CREATE TABLE images (
    id         TEXT PRIMARY KEY NOT NULL,
    hash       TEXT NOT NULL UNIQUE,
    format     TEXT NOT NULL,
    width      INTEGER NOT NULL,
    height     INTEGER NOT NULL,
    size_bytes INTEGER NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE jobs (
    id            TEXT PRIMARY KEY NOT NULL,
    kind          TEXT NOT NULL,
    status        TEXT NOT NULL,
    error_code    TEXT,
    error_message TEXT,
    created_at    TEXT NOT NULL,
    finished_at   TEXT
);
