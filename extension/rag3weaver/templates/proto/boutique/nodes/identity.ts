function run({ config }: { config: { key: string } }) {
  return { record: { key: config.key } };
}
