// Authentication — LTI (Learning Tools Interoperability) placeholder
export type LtiLaunch = {
  oauth_consumer_key: string;
  resource_link_id: string;
};

export function validateLtiLaunch(params: Record<string, string>): boolean {
  // TODO: validate LTI launch signature and params
  return false;
}
