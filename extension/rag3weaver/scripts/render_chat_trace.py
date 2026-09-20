#!/usr/bin/env python3
"""Render a benchmark's JSONL events as Markdown, keeping channels distinct."""
import argparse
import json
from pathlib import Path


def render(source):
    output = ["# Recorded agent trace\n",
              "Tool results are exactly the presentation sent to the model, including any truncation. "
              "Provider reasoning, when present, is separate from public messages.\n"]
    channel, fragments = None, []

    def flush():
        nonlocal channel, fragments
        if fragments:
            content = "".join(fragments)
            if channel == "reasoning":
                output.append("<details><summary>Provider reasoning</summary>\n\n" + content + "\n\n</details>\n")
            else:
                output.append("### Assistant\n\n" + content + "\n")
        channel, fragments = None, []

    def block(value):
        content = value if isinstance(value, str) else json.dumps(value, ensure_ascii=False, indent=2)
        fence = "`" * max(3, max((len(s) for s in content.split() if set(s) == {"`"}), default=0) + 1)
        return f"{fence}\n{content}\n{fence}\n"

    for line in source.read_text().splitlines():
        event = json.loads(line)
        kind = event.get("event")
        if kind in ("token", "reasoning"):
            if kind != channel:
                flush()
            channel = kind
            fragments.append(event.get("text", ""))
            continue
        flush()
        if kind == "generation_start":
            output.append(f"## Generation — {event.get('elapsed_seconds', 0):.2f} s\n")
        elif kind == "tool_start":
            arguments = event.get("arguments", "")
            try:
                arguments = json.loads(arguments)
            except (ValueError, TypeError):
                pass
            output.append(f"### Tool call: {event['name']} ({event['id']})\n\n" + block(arguments))
        elif kind == "tool_end":
            output.append(f"### Tool result: {event['name']} ({event['id']})\n\n" + block(event.get("content", "")))
        elif kind in ("generation_end", "generation_error", "done"):
            output.append(block(event))
    flush()
    return "\n".join(output)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("events", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    args.output.write_text(render(args.events))
