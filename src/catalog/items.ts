// Catalog — items placeholder (paid resources/items)
export type Item = { id: string; title: string; price: number };

export function listItems(): Item[] {
  // TODO: fetch items from storage
  return [];
}

export function getItem(id: string): Item | null {
  return null;
}
