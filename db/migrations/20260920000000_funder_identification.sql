-- migrate:up
ALTER TABLE exposed.funding_entries
    ADD COLUMN donor_status text,
    ADD COLUMN company_number text,
    ADD CONSTRAINT funding_entries_company_number_status
        CHECK (company_number IS NULL OR donor_status IS NOT DISTINCT FROM 'Company');

COMMENT ON COLUMN exposed.funding_entries.donor_status IS
    'Explicit Parliament DonorStatus for the attributed donor; NULL when unavailable';
COMMENT ON COLUMN exposed.funding_entries.company_number IS
    'Parliament DonorCompanyIdentifier when DonorStatus is Company; text preserves leading zeros';

-- migrate:down
ALTER TABLE exposed.funding_entries
    DROP CONSTRAINT funding_entries_company_number_status,
    DROP COLUMN company_number,
    DROP COLUMN donor_status;
