use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    pub id: i64,
    pub content_type: String,
    pub content: String,
    /// Plain-text version of `content` (for rich text clips); equals
    /// `content` for plain text / link / image clips.
    pub plain_content: Option<String>,
    /// Original RTF bytes of a rich text clip, base64-encoded; used to
    /// restore formatting when pasting.
    pub rtf: Option<String>,
    pub title: Option<String>,
    pub copied_at: i64,
    pub pinned: bool,
    /// Bundle id of the app the clip was copied from (e.g. com.jetbrains.goland).
    pub source_app: Option<String>,
    /// Localized name of that app (e.g. "GoLand").
    pub source_name: Option<String>,
    /// When the clip was pinned (None = not pinned). Pinned list orders by this.
    pub pinned_at: Option<i64>,
}

pub struct Db(Connection);

impl Db {
    pub fn open(app: &AppHandle) -> rusqlite::Result<Self> {
        let dir = app.path().app_data_dir().expect("app data dir");
        std::fs::create_dir_all(&dir).expect("create app data dir");
        let conn = Connection::open(dir.join("paste.db"))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS clips (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content_type TEXT NOT NULL DEFAULT 'text',
                content TEXT NOT NULL,
                plain_content TEXT NOT NULL DEFAULT '',
                rtf TEXT,
                title TEXT,
                copied_at INTEGER NOT NULL,
                pinned INTEGER NOT NULL DEFAULT 0,
                source_app TEXT,
                source_name TEXT,
                pinned_at INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_clips_copied_at ON clips(copied_at DESC);

            CREATE VIRTUAL TABLE IF NOT EXISTS clips_fts USING fts5(
                plain_content,
                content='clips',
                content_rowid='id',
                tokenize='trigram'
            );
            CREATE TRIGGER IF NOT EXISTS clips_ai AFTER INSERT ON clips BEGIN
                INSERT INTO clips_fts(rowid, plain_content) VALUES (new.id, new.plain_content);
            END;
            CREATE TRIGGER IF NOT EXISTS clips_ad AFTER DELETE ON clips BEGIN
                INSERT INTO clips_fts(clips_fts, rowid, plain_content)
                    VALUES ('delete', old.id, old.plain_content);
            END;
            CREATE TRIGGER IF NOT EXISTS clips_au AFTER UPDATE ON clips BEGIN
                INSERT INTO clips_fts(clips_fts, rowid, plain_content)
                    VALUES ('delete', old.id, old.plain_content);
                INSERT INTO clips_fts(rowid, plain_content) VALUES (new.id, new.plain_content);
            END;
            "#,
        )?;
        // Add the title column to databases created before it existed.
        let cols: Vec<String> = conn
            .prepare("PRAGMA table_info(clips)")?
            .query_map([], |r| r.get::<_, String>(1))?
            .collect::<rusqlite::Result<_>>()?;
        if !cols.iter().any(|c| c == "title") {
            conn.execute("ALTER TABLE clips ADD COLUMN title TEXT", [])?;
        }
        if !cols.iter().any(|c| c == "plain_content") {
            conn.execute(
                "ALTER TABLE clips ADD COLUMN plain_content TEXT NOT NULL DEFAULT ''",
                [],
            )?;
            // Existing rows predate rich text; their content is already plain.
            conn.execute("UPDATE clips SET plain_content = content", [])?;
        }
        if !cols.iter().any(|c| c == "rtf") {
            conn.execute("ALTER TABLE clips ADD COLUMN rtf TEXT", [])?;
        }
        if !cols.iter().any(|c| c == "source_app") {
            conn.execute("ALTER TABLE clips ADD COLUMN source_app TEXT", [])?;
        }
        if !cols.iter().any(|c| c == "source_name") {
            conn.execute("ALTER TABLE clips ADD COLUMN source_name TEXT", [])?;
        }
        if !cols.iter().any(|c| c == "pinned_at") {
            conn.execute("ALTER TABLE clips ADD COLUMN pinned_at INTEGER", [])?;
            // Clips pinned before pinned_at existed keep their pin time slot.
            conn.execute(
                "UPDATE clips SET pinned_at = copied_at WHERE pinned = 1",
                [],
            )?;
        }
        // Databases whose FTS still points at `content` (pre-rich-text
        // schema) need the index rebuilt against plain_content.
        let fts_cols: Vec<String> = conn
            .prepare("PRAGMA table_info(clips_fts)")?
            .query_map([], |r| r.get::<_, String>(1))?
            .collect::<rusqlite::Result<_>>()?;
        if !fts_cols.iter().any(|c| c == "plain_content") {
            conn.execute_batch(
                r#"
                DROP TRIGGER IF EXISTS clips_ai;
                DROP TRIGGER IF EXISTS clips_ad;
                DROP TRIGGER IF EXISTS clips_au;
                DROP TABLE IF EXISTS clips_fts;
                CREATE VIRTUAL TABLE clips_fts USING fts5(
                    plain_content,
                    content='clips',
                    content_rowid='id',
                    tokenize='trigram'
                );
                INSERT INTO clips_fts(rowid, plain_content)
                    SELECT id, plain_content FROM clips;
                CREATE TRIGGER clips_ai AFTER INSERT ON clips BEGIN
                    INSERT INTO clips_fts(rowid, plain_content) VALUES (new.id, new.plain_content);
                END;
                CREATE TRIGGER clips_ad AFTER DELETE ON clips BEGIN
                    INSERT INTO clips_fts(clips_fts, rowid, plain_content)
                        VALUES ('delete', old.id, old.plain_content);
                END;
                CREATE TRIGGER clips_au AFTER UPDATE ON clips BEGIN
                    INSERT INTO clips_fts(clips_fts, rowid, plain_content)
                        VALUES ('delete', old.id, old.plain_content);
                    INSERT INTO clips_fts(rowid, plain_content) VALUES (new.id, new.plain_content);
                END;
                "#,
            )?;
        }
        Ok(Self(conn))
    }

    fn row_to_clip(row: &rusqlite::Row) -> rusqlite::Result<Clip> {
        Ok(Clip {
            id: row.get(0)?,
            content_type: row.get(1)?,
            content: row.get(2)?,
            plain_content: row.get(3)?,
            rtf: row.get(4)?,
            title: row.get(5)?,
            copied_at: row.get(6)?,
            pinned: row.get::<_, i64>(7)? != 0,
            source_app: row.get(8)?,
            source_name: row.get(9)?,
            pinned_at: row.get(10)?,
        })
    }

    const CLIP_COLS: &'static str = "id, content_type, content, plain_content, rtf, title, \
                                     copied_at, pinned, source_app, source_name, pinned_at";

    /// Insert a clip. `plain_content` is the searchable plain-text form
    /// (equal to `content` for text / link / image); `rtf` holds the
    /// original RTF bytes (base64) of rich text clips for format-preserving
    /// paste. Returns the new row id, or None when it is a consecutive
    /// duplicate of the most recent clip.
    pub fn insert(
        &mut self,
        content_type: &str,
        content: &str,
        plain_content: &str,
        rtf: Option<&str>,
        title: Option<&str>,
        source_app: Option<&str>,
        source_name: Option<&str>,
    ) -> rusqlite::Result<Option<i64>> {
        let last: Option<(String, String)> = self
            .0
            .query_row(
                "SELECT content_type, content FROM clips ORDER BY copied_at DESC, id DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if last.as_ref().map(|(t, c)| (t.as_str(), c.as_str())) == Some((content_type, content)) {
            return Ok(None);
        }
        let copied_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_millis() as i64;
        self.0.execute(
            "INSERT INTO clips (content_type, content, plain_content, rtf, title, copied_at, source_app, source_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![content_type, content, plain_content, rtf, title, copied_at, source_app, source_name],
        )?;
        Ok(Some(self.0.last_insert_rowid()))
    }

    pub fn set_title(&self, id: i64, title: Option<&str>) -> rusqlite::Result<()> {
        self.0.execute(
            "UPDATE clips SET title = ?1 WHERE id = ?2",
            params![title, id],
        )?;
        Ok(())
    }

    pub fn recent(&self, limit: i64) -> rusqlite::Result<Vec<Clip>> {
        let mut stmt = self.0.prepare(&format!(
            "SELECT {cols}
             FROM clips
             ORDER BY copied_at DESC, id DESC
             LIMIT ?1",
            cols = Self::CLIP_COLS
        ))?;
        let rows = stmt.query_map(params![limit], Self::row_to_clip)?;
        rows.collect()
    }

    pub fn pinned_clips(&self, limit: i64) -> rusqlite::Result<Vec<Clip>> {
        let mut stmt = self.0.prepare(&format!(
            "SELECT {cols}
             FROM clips
             WHERE pinned = 1
             ORDER BY pinned_at DESC, id DESC
             LIMIT ?1",
            cols = Self::CLIP_COLS
        ))?;
        let rows = stmt.query_map(params![limit], Self::row_to_clip)?;
        rows.collect()
    }

    pub fn search(&self, query: &str, limit: i64) -> rusqlite::Result<Vec<Clip>> {
        if query.trim().is_empty() {
            return self.recent(limit);
        }
        // Trigram FTS5 needs 3+ characters; fall back to LIKE for short queries.
        let rows = if query.chars().count() >= 3 {
            let escaped = query.replace('"', "\"\"");
            let q = format!("\"{escaped}\"");
            let mut stmt = self.0.prepare(&format!(
                "SELECT c.{cols}
                 FROM clips_fts f
                 JOIN clips c ON c.id = f.rowid
                 WHERE clips_fts MATCH ?1
                 ORDER BY c.copied_at DESC, c.id DESC
                 LIMIT ?2",
                cols = Self::CLIP_COLS
            ))?;
            let results = stmt
                .query_map(params![q, limit], Self::row_to_clip)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            results
        } else {
            let pattern = format!("%{}%", query.replace(['%', '_', '\\'], "\\$0"));
            let mut stmt = self.0.prepare(&format!(
                "SELECT {cols}
                 FROM clips
                 WHERE plain_content LIKE ?1 ESCAPE '\\'
                 ORDER BY copied_at DESC, id DESC
                 LIMIT ?2",
                cols = Self::CLIP_COLS
            ))?;
            let results = stmt
                .query_map(params![pattern, limit], Self::row_to_clip)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            results
        };
        Ok(rows)
    }

    pub fn get_clip(&self, id: i64) -> rusqlite::Result<Option<Clip>> {
        self.0
            .query_row(
                &format!(
                    "SELECT {cols} FROM clips WHERE id = ?1",
                    cols = Self::CLIP_COLS
                ),
                params![id],
                Self::row_to_clip,
            )
            .optional()
    }

    pub fn delete(&mut self, id: i64) -> rusqlite::Result<()> {
        self.0
            .execute("DELETE FROM clips WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn toggle_pin(&mut self, id: i64) -> rusqlite::Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_millis() as i64;
        self.0.execute(
            "UPDATE clips
             SET pinned = NOT pinned,
                 pinned_at = CASE WHEN pinned = 0 THEN ?1 ELSE NULL END
             WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }
}
