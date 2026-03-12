// Analytics — events collection placeholder
export type Event = {
  name: string;
  payload: Record<string, unknown>;
  at: string;
};

export function trackEvent(
  name: string,
  payload: Record<string, unknown> = {},
) {
  const e: Event = { name, payload, at: new Date().toISOString() };
  // TODO: push to event store / queue
  return e;
}
