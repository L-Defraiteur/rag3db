interface Form {
  key: string;
  name: string;
  price: number;
  category: string;
}

function run({ config }: { config: Form }) {
  const record: Record<string, unknown> = { key: config.key, name: config.name, price: config.price };
  if (config.category !== "") {
    record.category = config.category;
  }
  return { record };
}
