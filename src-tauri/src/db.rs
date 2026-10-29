use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use tauri::{AppHandle, Manager};

/// Row id of the built-in "Pinned" pinboard (created by the migration).
pub const BUILTIN_PINNED_ID: i64 = 1;

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
    /// Owning pinboard (None = plain history item).
    pub pinboard_id: Option<i64>,
    /// Ordering inside the pinboard (insert time by default; drag-reorder later).
    pub display_order: Option<i64>,
    /// Extra type metadata as JSON (file count/size, image size, color hex).
    pub meta: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Pinboard {
    pub id: i64,
    pub name: String,
    /// Packed 0xRRGGBB color of the pinboard tab icon.
    pub color: i64,
    pub display_order: i64,
    pub created_at: i64,
}

pub struct Db(Connection);

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_millis() as i64
}

impl Db {
    pub fn open(app: &AppHandle) -> rusqlite::Result<Self> {
        let dir = app.path().app_data_dir().expect("app data dir");
        std::fs::create_dir_all(&dir).expect("create app data dir");
        let conn = Connection::open(dir.join("paste.db"))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        Self::init(&conn)?;
        Ok(Self(conn))
    }

    /// Create/migrate the schema. Separate from `open` so unit tests can run
    /// the real migration against an in-memory database.
    fn init(conn: &Connection) -> rusqlite::Result<()> {
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

            CREATE TABLE IF NOT EXISTS pinboards (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                color INTEGER NOT NULL DEFAULT 0,
                display_order INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL
            );

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
        // Add columns introduced after the first schema (older databases).
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
        if !cols.iter().any(|c| c == "pinboard_id") {
            conn.execute("ALTER TABLE clips ADD COLUMN pinboard_id INTEGER", [])?;
        }
        if !cols.iter().any(|c| c == "display_order") {
            conn.execute("ALTER TABLE clips ADD COLUMN display_order INTEGER", [])?;
        }
        if !cols.iter().any(|c| c == "meta") {
            conn.execute("ALTER TABLE clips ADD COLUMN meta TEXT", [])?;
        }
        // Built-in Pinned pinboard with the fixed well-known row id.
        let pinned_board: Option<i64> = conn
            .query_row(
                "SELECT id FROM pinboards WHERE id = ?1",
                params![BUILTIN_PINNED_ID],
                |r| r.get(0),
            )
            .optional()?;
        if pinned_board.is_none() {
            conn.execute(
                "INSERT INTO pinboards (id, name, color, display_order, created_at)
                 VALUES (?1, 'Pinned', ?2, 0, ?3)",
                params![BUILTIN_PINNED_ID, 0xef554d, now_ms()],
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
        Ok(())
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
            pinboard_id: row.get(11)?,
            display_order: row.get(12)?,
            meta: row.get(13)?,
        })
    }

    const CLIP_COLS: &'static str = "id, content_type, content, plain_content, rtf, title, \
                                     copied_at, pinned, source_app, source_name, pinned_at, \
                                     pinboard_id, display_order, meta";

    /// Panel/list queries select every CLIP_COLS position except the heavy
    /// `rtf` blob (kept as NULL to preserve row_to_clip's positional map):
    /// the panel never renders RTF — it keeps the HTML in `content` — while
    /// get_clip returns the full row for paste-back. Loading the multi-MB
    /// base64 blobs read SQLite overflow pages for nothing: view_clips
    /// measured 300ms–2s on a 31MB history, pushing the overlay into its
    /// 250ms fallback-reveal regime.
    const CLIP_COLS_LEAN: [&'static str; 14] = [
        "id",
        "content_type",
        "content",
        "plain_content",
        "NULL as rtf",
        "title",
        "copied_at",
        "pinned",
        "source_app",
        "source_name",
        "pinned_at",
        "pinboard_id",
        "display_order",
        "meta",
    ];

    /// Comma-joined column list; `prefix` (e.g. "c") qualifies plain columns
    /// for joined queries — expression columns like `NULL as rtf` must stay
    /// unqualified.
    fn cols_sql(prefix: Option<&str>) -> String {
        Self::CLIP_COLS_LEAN
            .iter()
            .map(|col| {
                if col.contains(' ') {
                    (*col).to_string()
                } else {
                    match prefix {
                        Some(p) => format!("{p}.{col}"),
                        None => (*col).to_string(),
                    }
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Insert a clip. `plain_content` is the searchable plain-text form
    /// (equal to `content` for text / link / image); `rtf` holds the
    /// original RTF bytes (base64) of rich text clips for format-preserving
    /// paste. Returns the new row id, or None when it is a consecutive
    /// duplicate of the most recent clip.
    #[allow(clippy::too_many_arguments)]
    pub fn insert(
        &mut self,
        content_type: &str,
        content: &str,
        plain_content: &str,
        rtf: Option<&str>,
        title: Option<&str>,
        meta: Option<&str>,
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
        let copied_at = now_ms();
        self.0.execute(
            "INSERT INTO clips (content_type, content, plain_content, rtf, title, meta, copied_at, source_app, source_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![content_type, content, plain_content, rtf, title, meta, copied_at, source_app, source_name],
        )?;
        Ok(Some(self.0.last_insert_rowid()))
    }

    /// Pasting makes the clip "current" again, like the original app: bump
    /// `copied_at` to now so the card returns to the front of the history,
    /// and drop other *visible-history* rows with identical content so the
    /// pasted card is never shown twice. Pinned duplicates and copies that
    /// live in a pinboard are kept — they are deliberate user states, not
    /// history clutter.
    pub fn touch_and_dedup(&mut self, id: i64) -> rusqlite::Result<()> {
        let key = self
            .0
            .query_row(
                "SELECT content_type, content FROM clips WHERE id = ?1",
                params![id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()?;
        let Some((content_type, content)) = key else {
            return Ok(());
        };
        self.0
            .execute("UPDATE clips SET copied_at = ?1 WHERE id = ?2", params![now_ms(), id])?;
        self.0.execute(
            "DELETE FROM clips
             WHERE id != ?1 AND content_type = ?2 AND content = ?3
               AND pinned = 0 AND pinboard_id IS NULL",
            params![id, content_type, content],
        )?;
        Ok(())
    }

    pub fn set_title(&self, id: i64, title: Option<&str>) -> rusqlite::Result<()> {
        self.0.execute(
            "UPDATE clips SET title = ?1 WHERE id = ?2",
            params![title, id],
        )?;
        Ok(())
    }

    /// Replace a clip's content (edit flow). The FTS triggers keep the
    /// search index in sync.
    pub fn update_clip(
        &self,
        id: i64,
        content: &str,
        plain_content: &str,
        rtf: Option<&str>,
    ) -> rusqlite::Result<()> {
        self.0.execute(
            "UPDATE clips SET content = ?1, plain_content = ?2, rtf = COALESCE(?3, rtf) WHERE id = ?4",
            params![content, plain_content, rtf, id],
        )?;
        Ok(())
    }

    /// Replace the stored image of an image clip (rotation flow).
    pub fn set_clip_content(&self, id: i64, content: &str) -> rusqlite::Result<()> {
        self.0.execute(
            "UPDATE clips SET content = ?1 WHERE id = ?2",
            params![content, id],
        )?;
        Ok(())
    }

    /// Insert a new text clip created by the user (Create Item flow).
    pub fn insert_text_clip(&mut self, text: &str) -> rusqlite::Result<Option<i64>> {
        self.insert("text", text, text, None, None, None, None, None)
    }

    pub fn recent(&self, limit: i64) -> rusqlite::Result<Vec<Clip>> {
        self.history(None, limit)
    }

    /// History view: items without a pinboard, pinned items floated to the top.
    pub fn history(&self, query: Option<&str>, limit: i64) -> rusqlite::Result<Vec<Clip>> {
        self.query_view("history", None, query, limit)
    }

    pub fn pinned_clips(&self, limit: i64) -> rusqlite::Result<Vec<Clip>> {
        self.query_view("pinned", None, None, limit)
    }

    pub fn search(&self, query: &str, limit: i64) -> rusqlite::Result<Vec<Clip>> {
        self.history(Some(query), limit)
    }

    /// View query shared by the panel tabs.
    /// view: "history" | "pinned" | "board" (board requires pinboard_id).
    pub fn query_view(
        &self,
        view: &str,
        pinboard_id: Option<i64>,
        query: Option<&str>,
        limit: i64,
    ) -> rusqlite::Result<Vec<Clip>> {
        let filter = match (view, pinboard_id) {
            ("pinned", _) => "c.pinned = 1".to_string(),
            ("board", Some(id)) => format!("c.pinboard_id = {id}"),
            _ => "c.pinboard_id IS NULL".to_string(),
        };
        let order = if view == "board" {
            "c.display_order ASC, c.copied_at DESC, c.id DESC"
        } else {
            "c.pinned DESC, CASE WHEN c.pinned = 1 THEN IFNULL(c.pinned_at, c.copied_at) \
             ELSE c.copied_at END DESC, c.id DESC"
        };
        let q = query.map(str::trim).filter(|q| !q.is_empty());
        let sql = match q {
            Some(q) if q.chars().count() >= 3 => format!(
                "SELECT {} FROM clips_fts f
                 JOIN clips c ON c.id = f.rowid
                 WHERE clips_fts MATCH ?1 AND {filter}
                 ORDER BY {order} LIMIT ?2",
                Self::cols_sql(Some("c"))
            ),
            Some(_) => format!(
                "SELECT {} FROM clips c
                 WHERE c.plain_content LIKE ?1 ESCAPE '\\' AND {filter}
                 ORDER BY {order} LIMIT ?2",
                Self::cols_sql(Some("c"))
            ),
            None => format!(
                "SELECT {} FROM clips c WHERE {filter} ORDER BY {order} LIMIT ?1",
                Self::cols_sql(None)
            ),
        };
        let mut stmt = self.0.prepare(&sql)?;
        let rows = match q {
            Some(q) if q.chars().count() >= 3 => {
                // Trigram FTS needs 3+ chars; phrase-quote the query.
                let escaped = format!("\"{}\"", q.replace('"', "\"\""));
                let rows = stmt.query_map(params![escaped, limit], Self::row_to_clip)?;
                rows.collect::<rusqlite::Result<Vec<_>>>()?
            }
            Some(q) => {
                let pattern = format!("%{}%", q.replace(['%', '_', '\\'], "\\$0"));
                let rows = stmt.query_map(params![pattern, limit], Self::row_to_clip)?;
                rows.collect::<rusqlite::Result<Vec<_>>>()?
            }
            None => {
                let rows = stmt.query_map(params![limit], Self::row_to_clip)?;
                rows.collect::<rusqlite::Result<Vec<_>>>()?
            }
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

    pub fn delete_many(&mut self, ids: &[i64]) -> rusqlite::Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let mut stmt = self.0.prepare("DELETE FROM clips WHERE id = ?1")?;
        for id in ids {
            stmt.execute(params![id])?;
        }
        Ok(())
    }

    /// Erase clipboard history but keep pinned items and pinboard content
    /// (matches the original "Erase History" behavior).
    pub fn erase_history(&mut self) -> rusqlite::Result<usize> {
        let n = self.0.execute(
            "DELETE FROM clips WHERE pinned = 0 AND pinboard_id IS NULL",
            [],
        )?;
        Ok(n)
    }

    /// Delete unpinned, non-pinboard clips older than the cutoff
    /// ("Keep History" retention). None = keep forever.
    pub fn enforce_capacity(&mut self, cutoff_ms: Option<i64>) -> rusqlite::Result<usize> {
        let Some(cutoff) = cutoff_ms else {
            return Ok(0);
        };
        let n = self.0.execute(
            "DELETE FROM clips WHERE pinned = 0 AND pinboard_id IS NULL AND copied_at < ?1",
            params![cutoff],
        )?;
        Ok(n)
    }

    /// Cutoff timestamp for a history capacity setting, in unix ms.
    pub fn capacity_cutoff(capacity: &str) -> Option<i64> {
        let now = now_ms();
        match capacity {
            "1day" => Some(now - 86_400_000),
            "1week" => Some(now - 7 * 86_400_000),
            "1month" => Some(now - 30 * 86_400_000),
            _ => None, // forever
        }
    }

    pub fn toggle_pin(&mut self, id: i64) -> rusqlite::Result<()> {
        let now = now_ms();
        self.0.execute(
            "UPDATE clips
             SET pinned = NOT pinned,
                 pinned_at = CASE WHEN pinned = 0 THEN ?1 ELSE NULL END
             WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    /// Move a clip into a pinboard (None returns it to plain history).
    pub fn move_clip_to_pinboard(
        &mut self,
        id: i64,
        pinboard_id: Option<i64>,
    ) -> rusqlite::Result<()> {
        let now = now_ms();
        self.0.execute(
            "UPDATE clips SET pinboard_id = ?1, display_order = ?2 WHERE id = ?3",
            params![pinboard_id, now, id],
        )?;
        Ok(())
    }

    // ---- pinboards ----

    pub fn list_pinboards(&self) -> rusqlite::Result<Vec<Pinboard>> {
        let mut stmt = self.0.prepare(
            "SELECT id, name, color, display_order, created_at
             FROM pinboards ORDER BY display_order ASC, id ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Pinboard {
                id: r.get(0)?,
                name: r.get(1)?,
                color: r.get(2)?,
                display_order: r.get(3)?,
                created_at: r.get(4)?,
            })
        })?;
        rows.collect()
    }

    pub fn create_pinboard(&mut self, name: &str, color: i64) -> rusqlite::Result<Pinboard> {
        let now = now_ms();
        let order: i64 = self
            .0
            .query_row(
                "SELECT IFNULL(MAX(display_order), 0) + 1 FROM pinboards",
                [],
                |r| r.get(0),
            )
            .unwrap_or(1);
        self.0.execute(
            "INSERT INTO pinboards (name, color, display_order, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![name, color, order, now],
        )?;
        let id = self.0.last_insert_rowid();
        Ok(Pinboard {
            id,
            name: name.to_string(),
            color,
            display_order: order,
            created_at: now,
        })
    }

    pub fn rename_pinboard(&mut self, id: i64, name: &str) -> rusqlite::Result<()> {
        self.0.execute(
            "UPDATE pinboards SET name = ?1 WHERE id = ?2",
            params![name, id],
        )?;
        Ok(())
    }

    pub fn set_pinboard_color(&mut self, id: i64, color: i64) -> rusqlite::Result<()> {
        self.0
            .execute("UPDATE pinboards SET color = ?1 WHERE id = ?2", params![color, id])?;
        Ok(())
    }

    /// Delete a pinboard together with its content (original behavior:
    /// "The Pinboard and all its content will be deleted"). The built-in
    /// Pinned board cannot be deleted.
    pub fn delete_pinboard(&mut self, id: i64) -> Result<(), String> {
        if id == BUILTIN_PINNED_ID {
            return Err("cannot delete the Pinned board".to_string());
        }
        self.0
            .execute("DELETE FROM clips WHERE pinboard_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        self.0
            .execute("DELETE FROM pinboards WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real schema on an in-memory database (same migration `open` runs).
    fn mem_db() -> Db {
        let conn = Connection::open_in_memory().unwrap();
        Db::init(&conn).unwrap();
        Db(conn)
    }

    fn insert(db: &mut Db, content: &str) -> i64 {
        db.insert("text", content, content, None, None, None, None, None)
            .unwrap()
            .expect("non-consecutive insert always stores")
    }

    #[test]
    fn touch_and_dedup_moves_pasted_clip_to_front_and_removes_duplicates() {
        let mut db = mem_db();
        let a = insert(&mut db, "A");
        std::thread::sleep(std::time::Duration::from_millis(3));
        let b = insert(&mut db, "B");
        std::thread::sleep(std::time::Duration::from_millis(3));
        let dup = insert(&mut db, "A"); // same content, copied again later
        std::thread::sleep(std::time::Duration::from_millis(3));
        let c = insert(&mut db, "C"); // separator: insert() dedups consecutives
        std::thread::sleep(std::time::Duration::from_millis(3));
        let pinned_dup = insert(&mut db, "A");
        db.toggle_pin(pinned_dup).unwrap();

        db.touch_and_dedup(a).unwrap();

        // History order: pinned items lead, then the freshly-pasted card,
        // then the rest by copied_at. The unpinned duplicate is gone; the
        // pinned duplicate survives.
        let ids: Vec<i64> = db.history(None, 10).unwrap().iter().map(|r| r.id).collect();
        assert_eq!(ids, vec![pinned_dup, a, c, b]);
        assert!(!ids.contains(&dup));
    }

    #[test]
    fn touch_and_dedup_keeps_pinboard_copies_and_other_content() {
        let mut db = mem_db();
        let a = insert(&mut db, "A");
        std::thread::sleep(std::time::Duration::from_millis(3));
        let b = insert(&mut db, "B");
        std::thread::sleep(std::time::Duration::from_millis(3));
        let board_copy = insert(&mut db, "A");
        let board = db.create_pinboard("Work", 0x333333).unwrap();
        db.move_clip_to_pinboard(board_copy, Some(board.id)).unwrap();

        db.touch_and_dedup(a).unwrap();

        let ids: Vec<i64> = db.history(None, 10).unwrap().iter().map(|r| r.id).collect();
        assert_eq!(ids, vec![a, b]);
        let board_rows = db.query_view("board", Some(board.id), None, 10).unwrap();
        assert_eq!(
            board_rows.iter().map(|r| r.id).collect::<Vec<_>>(),
            vec![board_copy]
        );
    }
}
