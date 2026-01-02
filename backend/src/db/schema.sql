-- Jobs table: Active and historical job records
CREATE TABLE IF NOT EXISTS jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    disc_label TEXT NOT NULL,
    title TEXT,
    year INTEGER,
    status TEXT NOT NULL,
    progress REAL DEFAULT 0.0,
    stage TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME,
    error TEXT
);

-- Rips table: Completed rip statistics
CREATE TABLE IF NOT EXISTS rips (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id INTEGER REFERENCES jobs(id),
    disc_id TEXT UNIQUE,
    output_path TEXT NOT NULL,
    original_size INTEGER,
    final_size INTEGER,
    duration_seconds INTEGER,
    completed_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Config table: Dynamic configuration storage
CREATE TABLE IF NOT EXISTS config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status);
CREATE INDEX IF NOT EXISTS idx_rips_disc_id ON rips(disc_id);
CREATE INDEX IF NOT EXISTS idx_jobs_created_at ON jobs(created_at);
