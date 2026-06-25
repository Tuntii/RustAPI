CREATE TABLE IF NOT EXISTS users (
    id          VARCHAR PRIMARY KEY,
    github_id   BIGINT NOT NULL,
    login       VARCHAR NOT NULL,
    avatar_url  VARCHAR,
    email       VARCHAR,
    tier        VARCHAR NOT NULL DEFAULT 'hobby',
    created_at  TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_users_github_id ON users (github_id);

CREATE TABLE IF NOT EXISTS oauth_devices (
    id          VARCHAR PRIMARY KEY,
    device_code VARCHAR NOT NULL UNIQUE,
    user_code   VARCHAR NOT NULL,
    user_id     VARCHAR REFERENCES users(id),
    client_id   VARCHAR NOT NULL DEFAULT 'rustapi-cli',
    scopes      VARCHAR NOT NULL DEFAULT 'user:read',
    expires_at  TIMESTAMP NOT NULL,
    created_at  TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_oauth_devices_device_code ON oauth_devices (device_code);
CREATE INDEX idx_oauth_devices_user_code ON oauth_devices (user_code);
