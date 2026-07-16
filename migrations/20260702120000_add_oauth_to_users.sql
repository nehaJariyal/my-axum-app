-- OAuth support: track which provider a user signed up with.
-- 'local' = email/password signup, otherwise provider name e.g. 'google'.
ALTER TABLE users ADD COLUMN IF NOT EXISTS provider TEXT NOT NULL DEFAULT 'local';

-- Stable unique id from the OAuth provider (e.g. Google `sub`). NULL for local users.
ALTER TABLE users ADD COLUMN IF NOT EXISTS provider_id TEXT;

-- One account per provider identity.
CREATE UNIQUE INDEX IF NOT EXISTS users_provider_provider_id_idx
    ON users (provider, provider_id)
    WHERE provider_id IS NOT NULL;
