export function searchQueryString(
  params: Pick<URLSearchParams, "get">,
): string {
  const retained = new URLSearchParams();
  const value = params.get("q");
  if (value !== null) retained.set("q", value);
  return retained.size ? `?${retained}` : "";
}
