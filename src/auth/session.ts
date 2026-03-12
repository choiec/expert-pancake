// Authentication — session management placeholder
export type Session = { id: string; userId: string; createdAt: string };

export function createSession(userId: string): Session {
  return { id: `${Date.now()}`, userId, createdAt: new Date().toISOString() };
}

export function destroySession(sessionId: string): void {
  // TODO: remove session from store
}
