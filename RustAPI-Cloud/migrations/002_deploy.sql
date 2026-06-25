CREATE TABLE IF NOT EXISTS projects (
    id          VARCHAR PRIMARY KEY,
    user_id     VARCHAR NOT NULL REFERENCES users(id),
    name        VARCHAR NOT NULL,
    created_at  TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_projects_user_name ON projects (user_id, name);

CREATE TABLE IF NOT EXISTS deploys (
    id              VARCHAR PRIMARY KEY,
    project_id      VARCHAR NOT NULL REFERENCES projects(id),
    user_id         VARCHAR NOT NULL REFERENCES users(id),
    binary_path     VARCHAR NOT NULL,
    status          VARCHAR NOT NULL DEFAULT 'queued',
    url             VARCHAR,
    port            INTEGER,
    pid             INTEGER,
    error_message   VARCHAR,
    created_at      TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_deploys_user_id ON deploys (user_id);
CREATE INDEX idx_deploys_project_id ON deploys (project_id);
CREATE INDEX idx_deploys_status ON deploys (status);