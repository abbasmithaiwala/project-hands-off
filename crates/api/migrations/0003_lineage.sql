ALTER TABLE images ADD COLUMN name TEXT;
ALTER TABLE images ADD COLUMN parent_id TEXT REFERENCES images(id) ON DELETE CASCADE;
ALTER TABLE images ADD COLUMN edit_json TEXT;

CREATE INDEX images_parent_id ON images(parent_id);
CREATE INDEX images_created_at ON images(created_at DESC);
CREATE INDEX jobs_finished_at ON jobs(finished_at);
