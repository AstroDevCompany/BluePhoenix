-- Neon / PostgreSQL schema. Authorization is enforced in Axum; RLS is defense in depth.

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS users (
  id UUID PRIMARY KEY,
  email TEXT NOT NULL UNIQUE,
  password_hash TEXT NOT NULL,
  display_name TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  revision BIGINT NOT NULL DEFAULT 1,
  deleted_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS refresh_tokens (
  id UUID PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  token_hash TEXT NOT NULL,
  expires_at TIMESTAMPTZ NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  revoked_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS password_reset_tokens (
  id UUID PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  token_hash TEXT NOT NULL,
  expires_at TIMESTAMPTZ NOT NULL,
  used_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS devices (
  id UUID PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  hostname TEXT,
  os TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS encrypted_secrets (
  id UUID PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  kind TEXT NOT NULL,
  ciphertext TEXT NOT NULL,
  nonce TEXT NOT NULL,
  wrap_params JSONB NOT NULL DEFAULT '{}'::jsonb,
  revision BIGINT NOT NULL DEFAULT 1,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  deleted_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX IF NOT EXISTS encrypted_secrets_user_kind
  ON encrypted_secrets(user_id, kind) WHERE deleted_at IS NULL;

CREATE TABLE IF NOT EXISTS sync_rows (
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  table_name TEXT NOT NULL,
  row_id UUID NOT NULL,
  payload JSONB NOT NULL,
  revision BIGINT NOT NULL DEFAULT 1,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  deleted_at TIMESTAMPTZ,
  PRIMARY KEY (user_id, table_name, row_id)
);

CREATE INDEX IF NOT EXISTS sync_rows_updated ON sync_rows(user_id, updated_at);

CREATE OR REPLACE FUNCTION notify_user_changes() RETURNS trigger AS $$
BEGIN
  PERFORM pg_notify('user_changes', NEW.user_id::text);
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS sync_rows_notify ON sync_rows;
CREATE TRIGGER sync_rows_notify AFTER INSERT OR UPDATE ON sync_rows
FOR EACH ROW EXECUTE FUNCTION notify_user_changes();

CREATE OR REPLACE FUNCTION notify_user_secret_changes() RETURNS trigger AS $$
BEGIN
  PERFORM pg_notify('user_secret_changes', NEW.user_id::text);
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS encrypted_secrets_notify ON encrypted_secrets;
CREATE TRIGGER encrypted_secrets_notify AFTER INSERT OR UPDATE ON encrypted_secrets
FOR EACH ROW EXECUTE FUNCTION notify_user_secret_changes();
