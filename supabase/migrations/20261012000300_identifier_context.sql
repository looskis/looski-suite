-- Preserve phone extension and explicit parsing context on the relationship.
ALTER TABLE contacts.contact_identifiers ADD COLUMN extension text CHECK (length(extension) BETWEEN 1 AND 20);
ALTER TABLE contacts.contact_identifiers ADD COLUMN country text CHECK (country ~ '^[A-Z]{2}$');
UPDATE suite.schema_version SET version=4;
