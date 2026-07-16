-- Add migration script here
ALTER TABLE users ADD COLUMN password TEXT NOT NULL DEFAULT '';
