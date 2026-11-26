# An Upsert That Fires a Trigger Turns `INSERT OR IGNORE` in the Trigger Body into a Failure

**Source: Verified** (sqlite3 3.51.0 / 2026-11-24; SQLite documentation, sqlite.org/lang_createtrigger.html and sqlite.org/lang_upsert.html / 2026-11-24)

One item of [knowledge.md](../knowledge.md).

## Fact

When `INSERT ... ON CONFLICT (...) DO UPDATE` updates an existing row and so fires an `AFTER UPDATE` trigger, an `INSERT OR IGNORE` in the trigger's body that hits a uniqueness constraint fails with `UNIQUE constraint failed` instead of doing nothing.
The same trigger fired by a plain `UPDATE` ignores the conflict as written.
An `INSERT ... ON CONFLICT (...) DO UPDATE` in the trigger's body handles the same conflict in both cases.

## Basis

On 2026-11-24 we ran the following in sqlite3 3.51.0 on a new file.

```sql
CREATE TABLE rows (id TEXT PRIMARY KEY, month TEXT NOT NULL);
CREATE TABLE counts (month TEXT PRIMARY KEY, count INTEGER NOT NULL);
CREATE TRIGGER rows_update AFTER UPDATE OF month ON rows WHEN OLD.month <> NEW.month BEGIN
    UPDATE counts SET count = count - 1 WHERE month = OLD.month;
    INSERT OR IGNORE INTO counts (month, count) VALUES (NEW.month, 0);
    UPDATE counts SET count = count + 1 WHERE month = NEW.month;
END;
INSERT INTO counts VALUES ('2024-01', 1), ('2024-02', 1);
INSERT INTO rows VALUES ('a', '2024-01'), ('b', '2024-02');
UPDATE rows SET month = '2024-02' WHERE id = 'a';
INSERT INTO rows VALUES ('a', '2024-01') ON CONFLICT (id) DO UPDATE SET month = excluded.month;
```

The plain `UPDATE` moved the row into a month with a count and left the counts at 0 and 2.
The upsert that moved it back failed with `UNIQUE constraint failed: counts.month (19)` and left the counts unchanged.

We then ran the same statements on another new file with the trigger's last two statements replaced by one upsert.

```sql
INSERT INTO counts (month, count) VALUES (NEW.month, 1) ON CONFLICT (month) DO UPDATE SET count = count + 1;
```

The plain `UPDATE` left the counts at 0 and 2, and the upsert succeeded and left them at 1 and 1.

The daemon's tests of the timeline index failed the same way with the SQLite 3.46.0 that `libsqlite3-sys` 0.30.1 bundles, until the triggers raised their counts by an upsert.

The SQLite documentation describes the two rules this follows from.

- lang_createtrigger.html: "However if an ON CONFLICT clause is specified as part of the statement causing the trigger to fire, then conflict handling policy of the outer statement is used instead."
- lang_upsert.html: "The conflict resolution algorithm for the update operation of the DO UPDATE clause is always ABORT. [...] This is true even if the DO UPDATE clause is contained within an INSERT statement or a trigger that specifies some other conflict resolution algorithm."

## Implication

The triggers that keep the per-month counts of the timeline index raise a count with an upsert, since a row is written by an upsert that can move it into another month, kind, or local file ([Timeline Design §5](../design/timeline.md#5-the-timeline-index)).
