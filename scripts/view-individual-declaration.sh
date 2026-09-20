#!/usr/bin/env bash
# Fetch a concrete published example directly from Parliament; requires curl and jq.
set -euo pipefail

if [[ $# -gt 1 || ( $# -eq 1 && "$1" != "--raw" ) ]]; then
  echo "Usage: $0 [--raw]" >&2
  exit 2
fi
command -v curl >/dev/null || { echo "curl is required" >&2; exit 1; }
command -v jq >/dev/null || { echo "jq is required" >&2; exit 1; }

url='https://interests-api.parliament.uk/api/v2/Interests/16901'
response=$(curl --fail --silent --show-error --connect-timeout 10 --max-time 60 --retry 2 "$url")
if [[ "${1:-}" == "--raw" ]]; then
  jq . <<< "$response"
else
  jq --exit-status --arg url "$url" '
    . as $interest
    | (.versions | max_by(.register.publishedDate)) as $version
    | ($version.fields | map({key: .name, value: .value}) | from_entries) as $fields
    | if $fields.DonorStatus != "Individual" then
        error("Example donor status has changed; inspect with --raw")
      else {
        id: $interest.id,
        source: $url,
        member: $interest.registrant.memberDetail.nameDisplayAs,
        category: $interest.category.name,
        summary: $version.summary,
        registration_date: $version.registrationDate,
        register_published_date: $version.register.publishedDate,
        donor: $fields.DonorName,
        donor_status: $fields.DonorStatus,
        company_number: $fields.DonorCompanyIdentifier,
        amount: $fields.Value,
        currency: ([$version.fields[] | select(.name == "Value") | .typeInfo.currencyCode][0]),
        payment_type: $fields.PaymentType,
        donation_source: $fields.DonationSource,
        description: $fields.PaymentDescription
      } end
  ' <<< "$response"
fi
