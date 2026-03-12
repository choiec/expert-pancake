// Authentication — token utilities (JWT, opaque tokens)
export type TokenPayload = { sub: string; iat?: number; exp?: number };

export function signToken(payload: TokenPayload): string {
  // TODO: sign and return token
  return "token-not-implemented";
}

export function verifyToken(token: string): TokenPayload | null {
  // TODO: verify token
  return null;
}
