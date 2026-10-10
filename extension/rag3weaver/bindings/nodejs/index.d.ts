export type Json = null | boolean | number | string | Json[] | { [key: string]: Json };

export class Backend {
  static open(manifestPath: string, options?: { binary?: string; env?: NodeJS.ProcessEnv }): Promise<Backend>;
  describe(): Promise<Json>;
  call(name: string, args?: Json): Promise<Json>;
  journal(events: Json[]): Promise<Json>;
  journalRead(conversation: string, sinceMs?: number): Promise<Json>;
  indexState(): Promise<Json>;
  shutdown(): Promise<{ code: number | null; signal: string | null }>;
  readonly manifestPath: string;
}
export function binaryPath(): string;
export function vectorExtensionPath(): string;
export function templatesDir(): string;
export function platformKey(): string;
export function prepareManifest(templatePath: string, overrides?: Record<string, unknown>): Record<string, unknown>;
