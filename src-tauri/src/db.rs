use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
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

/// What makes two stored clips "the same content": the type and the stored
/// content, nothing else. Title, source app and time are presentation and
/// may differ between two copies of the same thing.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
struct ClipIdentity<'a> {
    content_type: &'a str,
    content: &'a str,
    /// `content` holds only the first path of a multi-file copy; the joined
    /// path list is what tells different file sets apart.
    plain_content: &'a str,
    /// Color clips all share the placeholder content, so the archived
    /// NSColor bytes kept in `rtf` are what tells two swatches apart. None
    /// for every other type: two copies of the same rich text may carry
    /// different RTF and are still the same card.
    rtf: Option<&'a str>,
}

impl<'a> ClipIdentity<'a> {
    fn new(
        content_type: &'a str,
        content: &'a str,
        plain_content: &'a str,
        rtf: Option<&'a str>,
    ) -> Self {
        Self {
            content_type,
            content,
            plain_content,
            rtf: if content_type == "color" { rtf } else { None },
        }
    }
}

/// A `ClipIdentity` read from a row, in `IDENTITY_COLS` order.
type OwnedIdentity = (String, String, String, Option<String>);

/// SQL: a `clips` row has the identity bound to ?1–?4 in `ClipIdentity`
/// field order. The byte-length test comes before the content comparison:
/// it reads the record header only, so rows of a different size — all but
/// the real candidates — are rejected without loading their content.
const SAME_CONTENT: &str = "content_type = ?1
                             AND octet_length(content) = octet_length(?2)
                             AND content = ?2 AND plain_content = ?3
                             AND (?4 IS NULL OR rtf IS ?4)";

/// `SAME_CONTENT` as a column list: read a clip's identity, or group clips
/// by it.
const IDENTITY_COLS: &str = "content_type, content, plain_content,
                             CASE WHEN content_type = 'color' THEN rtf END";

/// Delete every other *visible-history* clip with the same identity as the
/// kept one, so the same card is never shown twice. Pinned duplicates and
/// copies that live in a pinboard are kept — they are deliberate user
/// states, not history clutter.
fn delete_history_duplicates(
    conn: &Connection,
    keep_id: i64,
    identity: &ClipIdentity<'_>,
) -> rusqlite::Result<usize> {
    let ClipIdentity {
        content_type,
        content,
        plain_content,
        rtf,
    } = *identity;
    conn.execute(
        &format!(
            "DELETE FROM clips
             WHERE pinned = 0 AND pinboard_id IS NULL AND id != ?5
               AND {SAME_CONTENT}"
        ),
        params![content_type, content, plain_content, rtf, keep_id],
    )
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
        Self::purge_legacy_duplicates(conn)?;
        Ok(())
    }

    /// Histories written before `insert` deduplicated against the whole
    /// history hold the same content several times. Drop every older
    /// visible-history copy once, keeping the newest card of each (pinned
    /// and pinboard copies are never touched). `user_version` records that
    /// it ran.
    fn purge_legacy_duplicates(conn: &Connection) -> rusqlite::Result<()> {
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version >= 1 {
            return Ok(());
        }
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            &format!(
                "DELETE FROM clips WHERE id IN (
                     SELECT id FROM (
                         SELECT id,
                                ROW_NUMBER() OVER (
                                    PARTITION BY {IDENTITY_COLS}
                                    ORDER BY copied_at DESC, id DESC
                                ) AS copy_no
                         FROM clips
                         WHERE pinned = 0 AND pinboard_id IS NULL
                     )
                     WHERE copy_no > 1
                 )"
            ),
            [],
        )?;
        tx.pragma_update(None, "user_version", 1)?;
        tx.commit()
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
    /// paste.
    ///
    /// The same content already in the history is replaced: the new clip
    /// becomes the newest card and the older copies are deleted, so a card
    /// is never shown twice. Returns the new row id, or None when the clip
    /// equals the most recent one.
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
        let identity = ClipIdentity::new(content_type, content, plain_content, rtf);
        // A copy that equals the most recent clip changes nothing: its card
        // is already the newest one. The watcher also relies on this for
        // its first poll after a launch, which re-reads whatever the
        // pasteboard still holds.
        let is_most_recent: bool = self.0.query_row(
            &format!(
                "SELECT EXISTS(SELECT 1 FROM clips
                 WHERE id = (SELECT id FROM clips ORDER BY copied_at DESC, id DESC LIMIT 1)
                   AND {SAME_CONTENT})"
            ),
            params![content_type, content, plain_content, identity.rtf],
            |r| r.get(0),
        )?;
        if is_most_recent {
            return Ok(None);
        }
        // Add the newest card and drop the older copies in one step, so a
        // crash in between can never leave the same card twice.
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let copied_at = now_ms();
        tx.execute(
            "INSERT INTO clips (content_type, content, plain_content, rtf, title, meta, copied_at, source_app, source_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![content_type, content, plain_content, rtf, title, meta, copied_at, source_app, source_name],
        )?;
        let id = tx.last_insert_rowid();
        delete_history_duplicates(&tx, id, &identity)?;
        tx.commit()?;
        Ok(Some(id))
    }

    /// The `IDENTITY_COLS` of one clip, None when it no longer exists.
    fn identity_of(&self, id: i64) -> rusqlite::Result<Option<OwnedIdentity>> {
        self.0
            .query_row(
                &format!("SELECT {IDENTITY_COLS} FROM clips WHERE id = ?1"),
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()
    }

    /// Pasting makes the clip "current" again, like the original app: bump
    /// `copied_at` to now so the card returns to the front of the history,
    /// and drop other *visible-history* clips with the same content so the
    /// pasted card is never shown twice (same rule as `insert`).
    pub fn touch_and_dedup(&mut self, id: i64) -> rusqlite::Result<()> {
        let Some(key) = self.identity_of(id)? else {
            return Ok(());
        };
        let identity = ClipIdentity::new(&key.0, &key.1, &key.2, key.3.as_deref());
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "UPDATE clips SET copied_at = ?1 WHERE id = ?2",
            params![now_ms(), id],
        )?;
        delete_history_duplicates(&tx, id, &identity)?;
        tx.commit()?;
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

    /// Newest-first ids of the plain history (same order the panel shows).
    fn history_ids(db: &Db) -> Vec<i64> {
        db.history(None, 10).unwrap().iter().map(|r| r.id).collect()
    }

    fn insert_text(
        db: &mut Db,
        content: &str,
        source_app: Option<&str>,
    ) -> rusqlite::Result<Option<i64>> {
        db.insert("text", content, content, None, None, None, source_app, None)
    }

    /// Stored unconditionally: anything but a copy of the most recent clip.
    fn insert(db: &mut Db, content: &str) -> i64 {
        insert_text(db, content, None)
            .unwrap()
            .expect("anything but a copy of the most recent clip is stored")
    }

    /// Fix a clip's time so the history order does not depend on how fast
    /// the test happens to run.
    fn set_copied_at(db: &Db, id: i64, copied_at: i64) {
        db.0.execute(
            "UPDATE clips SET copied_at = ?1 WHERE id = ?2",
            params![copied_at, id],
        )
        .unwrap();
    }

    /// Store a clip the way versions before history-wide dedup did — a
    /// plain row nothing deduplicates. Older databases still hold these.
    fn legacy_row(
        db: &Db,
        content_type: &str,
        content: &str,
        plain_content: &str,
        rtf: Option<&str>,
        copied_at: i64,
    ) -> i64 {
        db.0.execute(
            "INSERT INTO clips (content_type, content, plain_content, rtf, copied_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![content_type, content, plain_content, rtf, copied_at],
        )
        .unwrap();
        db.0.last_insert_rowid()
    }

    /// The clip the search index still lists for `query` (by rowid).
    fn indexed_rowids(db: &Db, query: &str) -> Vec<i64> {
        db.0.prepare("SELECT rowid FROM clips_fts WHERE clips_fts MATCH ?1")
            .unwrap()
            .query_map(params![query], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    #[test]
    fn insert_replaces_an_older_duplicate_with_the_newest_card() {
        let mut db = mem_db();
        let t = now_ms() - 10_000;
        let a = insert_text(&mut db, "hello", Some("one.app"))
            .unwrap()
            .unwrap();
        set_copied_at(&db, a, t);
        let b = insert(&mut db, "world");
        set_copied_at(&db, b, t + 1);

        // "hello" copied again later, from another app: the card of this
        // newest copy goes to the front and the old one is deleted.
        let a2 = insert_text(&mut db, "hello", Some("two.app"))
            .unwrap()
            .unwrap();

        assert_eq!(history_ids(&db), vec![a2, b]);
        assert!(db.get_clip(a).unwrap().is_none());
        let newest = db.get_clip(a2).unwrap().unwrap();
        assert_eq!(newest.source_app.as_deref(), Some("two.app"));
        assert!(newest.copied_at > t + 1);
        assert_eq!(indexed_rowids(&db, "hello"), vec![a2]);
    }

    #[test]
    fn insert_leaves_a_copy_of_the_most_recent_clip_alone() {
        let mut db = mem_db();
        let a = insert_text(&mut db, "hello", Some("one.app"))
            .unwrap()
            .unwrap();

        // Same as the most recent clip: nothing changes. The watcher's first
        // poll after a launch re-reads whatever the pasteboard still holds,
        // which must not re-create that card (new id, new time, wrong app).
        assert_eq!(
            insert_text(&mut db, "hello", Some("two.app")).unwrap(),
            None
        );

        assert_eq!(history_ids(&db), vec![a]);
        let kept = db.get_clip(a).unwrap().unwrap();
        assert_eq!(kept.source_app.as_deref(), Some("one.app"));
    }

    #[test]
    fn insert_keeps_pinned_and_pinboard_copies_of_the_same_content() {
        let mut db = mem_db();
        let t = now_ms() - 10_000;
        let a = legacy_row(&db, "text", "A", "A", None, t);
        let pinned = legacy_row(&db, "text", "A", "A", None, t + 1);
        db.toggle_pin(pinned).unwrap();
        let board_copy = legacy_row(&db, "text", "A", "A", None, t + 2);
        let board = db.create_pinboard("Work", 0x333333).unwrap();
        db.move_clip_to_pinboard(board_copy, Some(board.id))
            .unwrap();
        // Separator, so the next copy is not one of the most recent clip.
        let b = legacy_row(&db, "text", "B", "B", None, t + 3);

        let a2 = insert(&mut db, "A");

        // Only the plain-history copy is replaced; pinned and pinboard
        // copies are deliberate user states and stay.
        assert_eq!(history_ids(&db), vec![pinned, a2, b]);
        assert!(db.get_clip(a).unwrap().is_none());
        let board_rows = db.query_view("board", Some(board.id), None, 10).unwrap();
        assert_eq!(
            board_rows.iter().map(|r| r.id).collect::<Vec<_>>(),
            vec![board_copy]
        );
    }

    #[test]
    fn insert_removes_every_older_copy() {
        let mut db = mem_db();
        let t = now_ms() - 10_000;
        let a1 = legacy_row(&db, "text", "alpha", "alpha", None, t);
        let a2 = legacy_row(&db, "text", "alpha", "alpha", None, t + 1);
        let b = legacy_row(&db, "text", "beta", "beta", None, t + 2);

        let newest = insert(&mut db, "alpha");

        assert_eq!(history_ids(&db), vec![newest, b]);
        assert!(db.get_clip(a1).unwrap().is_none());
        assert!(db.get_clip(a2).unwrap().is_none());
        assert_eq!(indexed_rowids(&db, "alpha"), vec![newest]);
    }

    #[test]
    fn insert_tells_multi_file_copies_apart() {
        let mut db = mem_db();
        let ab = db
            .insert(
                "file",
                "/tmp/a.txt",
                "/tmp/a.txt\n/tmp/b.txt",
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap()
            .unwrap();

        // Same first file (all `content` holds), different selection: not
        // the same content.
        let ac = db
            .insert(
                "file",
                "/tmp/a.txt",
                "/tmp/a.txt\n/tmp/c.txt",
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap()
            .unwrap();
        assert_eq!(history_ids(&db), vec![ac, ab]);

        // The same selection copied again replaces only its own old card.
        let ab2 = db
            .insert(
                "file",
                "/tmp/a.txt",
                "/tmp/a.txt\n/tmp/b.txt",
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap()
            .unwrap();
        assert_eq!(history_ids(&db), vec![ab2, ac]);
    }

    #[test]
    fn insert_tells_colors_apart() {
        let mut db = mem_db();
        let color = |db: &mut Db, archived: &str| {
            db.insert(
                "color",
                "#4da3ff",
                "#4da3ff",
                Some(archived),
                None,
                Some("{\"color\":\"#4da3ff\"}"),
                None,
                None,
            )
            .unwrap()
            .expect("a color is stored")
        };
        let red = color(&mut db, "archived-red");

        // Every color clip shares the placeholder content: a different
        // color right after it is still a new clip, not a duplicate.
        let blue = color(&mut db, "archived-blue");
        assert_eq!(history_ids(&db), vec![blue, red]);

        let text = insert(&mut db, "A");
        let red2 = color(&mut db, "archived-red");
        assert_eq!(history_ids(&db), vec![red2, text, blue]);
    }

    #[test]
    fn touch_and_dedup_moves_pasted_clip_to_front_and_removes_duplicates() {
        let mut db = mem_db();
        let t = now_ms() - 10_000;
        let b = legacy_row(&db, "text", "B", "B", None, t);
        let a = legacy_row(&db, "text", "A", "A", None, t + 1);
        let dup = legacy_row(&db, "text", "A", "A", None, t + 2); // copied again later
        let c = legacy_row(&db, "text", "C", "C", None, t + 3);
        let pinned_dup = legacy_row(&db, "text", "A", "A", None, t + 4);
        db.toggle_pin(pinned_dup).unwrap();

        db.touch_and_dedup(a).unwrap();

        // History order: pinned items lead, then the freshly-pasted card,
        // then the rest by copied_at. The unpinned duplicate is gone; the
        // pinned duplicate survives.
        let ids = history_ids(&db);
        assert_eq!(ids, vec![pinned_dup, a, c, b]);
        assert!(db.get_clip(dup).unwrap().is_none());
    }

    #[test]
    fn touch_and_dedup_spares_multi_file_copies_sharing_the_first_path() {
        let mut db = mem_db();
        let t = now_ms() - 10_000;
        let ab = db
            .insert(
                "file",
                "/tmp/a.txt",
                "/tmp/a.txt\n/tmp/b.txt",
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap()
            .unwrap();
        let ac = db
            .insert(
                "file",
                "/tmp/a.txt",
                "/tmp/a.txt\n/tmp/c.txt",
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap()
            .unwrap();
        set_copied_at(&db, ab, t);
        set_copied_at(&db, ac, t + 1);

        db.touch_and_dedup(ab).unwrap();

        // Pasting one selection must not delete a different selection that
        // merely starts with the same file; it just moves to the front.
        assert_eq!(history_ids(&db), vec![ab, ac]);
    }

    #[test]
    fn touch_and_dedup_keeps_pinboard_copies_and_other_content() {
        let mut db = mem_db();
        let board_copy = insert(&mut db, "A");
        let board = db.create_pinboard("Work", 0x333333).unwrap();
        db.move_clip_to_pinboard(board_copy, Some(board.id))
            .unwrap();
        let b = insert(&mut db, "B");
        let a = insert(&mut db, "A");

        db.touch_and_dedup(a).unwrap();

        assert_eq!(history_ids(&db), vec![a, b]);
        let board_rows = db.query_view("board", Some(board.id), None, 10).unwrap();
        assert_eq!(
            board_rows.iter().map(|r| r.id).collect::<Vec<_>>(),
            vec![board_copy]
        );
    }

    #[test]
    fn init_purges_duplicates_left_by_older_versions() {
        let mut db = mem_db();
        let t = now_ms() - 10_000;
        let a1 = legacy_row(&db, "text", "alpha", "alpha", None, t);
        let b = legacy_row(&db, "text", "beta", "beta", None, t + 1);
        let a2 = legacy_row(&db, "text", "alpha", "alpha", None, t + 2);
        let pinned_a = legacy_row(&db, "text", "alpha", "alpha", None, t + 3);
        db.toggle_pin(pinned_a).unwrap();
        let board_a = legacy_row(&db, "text", "alpha", "alpha", None, t + 4);
        let board = db.create_pinboard("Work", 0x333333).unwrap();
        db.move_clip_to_pinboard(board_a, Some(board.id)).unwrap();
        // A database last written before the purge has not recorded it.
        db.0.pragma_update(None, "user_version", 0).unwrap();

        Db::init(&db.0).unwrap();

        // One card per content: the newest copy of "alpha" stays, the older
        // one goes; pinned and pinboard copies are untouched.
        assert_eq!(history_ids(&db), vec![pinned_a, a2, b]);
        assert!(db.get_clip(a1).unwrap().is_none());
        let board_rows = db.query_view("board", Some(board.id), None, 10).unwrap();
        assert_eq!(
            board_rows.iter().map(|r| r.id).collect::<Vec<_>>(),
            vec![board_a]
        );
        // The search index dropped the purged row and nothing else.
        let mut indexed = indexed_rowids(&db, "alpha");
        indexed.sort_unstable();
        assert_eq!(indexed, vec![a2, pinned_a, board_a]);
        // Recorded as done, so a later launch does not repeat it.
        let version: i64 =
            db.0.query_row("PRAGMA user_version", [], |r| r.get(0))
                .unwrap();
        assert_eq!(version, 1);
    }

    /// Not part of the normal run (it builds a 40 MB history first):
    /// `cargo test --lib -- --ignored --nocapture dedup_cost`. Prints what
    /// history-wide dedup costs on a large history — per copy, and for the
    /// one-time cleanup of an older database. Plain text is kept short so
    /// building the trigram search index does not dominate the setup; the
    /// scan reads `content`, which is what carries the weight.
    #[test]
    #[ignore]
    fn dedup_cost_on_a_large_history() {
        use std::time::Instant;

        let mut db = mem_db();
        let filler = "lorem ipsum dolor sit amet ".repeat(100); // 2.7 KB
        let t = now_ms() - 10_000_000;
        let mut rows = 0;
        for i in 0..5_000 {
            // Every 10th row is an older copy of row 0.
            let copy_of = if i % 10 == 0 { 0 } else { i };
            legacy_row(
                &db,
                "text",
                &format!("{copy_of:06} {filler}"),
                "short",
                None,
                t + rows,
            );
            rows += 1;
        }
        // A few rich-text sized cards (long inline styles).
        for i in 0..20 {
            legacy_row(
                &db,
                "richtext",
                &format!("{i}{}", filler.repeat(370)), // 1 MB
                "short",
                None,
                t + rows,
            );
            rows += 1;
        }
        let mb =
            |sql: &str| -> f64 { db.0.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap() as f64 };
        eprintln!(
            "history: {rows} rows, {:.1} MB (in-memory)",
            mb("PRAGMA page_count") * mb("PRAGMA page_size") / 1e6
        );

        db.0.pragma_update(None, "user_version", 0).unwrap();
        let start = Instant::now();
        Db::init(&db.0).unwrap();
        eprintln!(
            "one-time cleanup of an older database: {:.0} ms",
            start.elapsed().as_secs_f64() * 1e3
        );

        let start = Instant::now();
        db.insert(
            "text",
            "fresh copy",
            "fresh copy",
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap()
        .unwrap();
        eprintln!(
            "copy of content not in the history: {:.1} ms",
            start.elapsed().as_secs_f64() * 1e3
        );

        let start = Instant::now();
        db.insert(
            "text",
            &format!("{:06} {filler}", 4_321),
            "short",
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap()
        .unwrap();
        eprintln!(
            "copy of content already in the history: {:.1} ms",
            start.elapsed().as_secs_f64() * 1e3
        );
    }
}
