//! SQLite is the authoritative owner of account, character, ticket and
//! persistence rows. The zone has no write access to the database directly.
use rusqlite::Connection;

pub fn initialize(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA foreign_keys=ON;
         PRAGMA busy_timeout=5000;

         CREATE TABLE IF NOT EXISTS accounts (
             id TEXT PRIMARY KEY NOT NULL,
             discord_id TEXT NOT NULL UNIQUE,
             display_name TEXT NOT NULL,
             created_at INTEGER NOT NULL,
             disabled INTEGER NOT NULL DEFAULT 0
         );

         CREATE TABLE IF NOT EXISTS auth_flows (
             id TEXT PRIMARY KEY NOT NULL,
             state_hash TEXT NOT NULL UNIQUE,
             poll_hash TEXT NOT NULL UNIQUE,
             code_verifier TEXT NOT NULL,
             phase TEXT NOT NULL CHECK(phase IN ('pending','exchanging','complete','consumed')),
             session_result TEXT,
             expires_at INTEGER NOT NULL
         );

         CREATE TABLE IF NOT EXISTS sessions (
             token_hash TEXT PRIMARY KEY NOT NULL,
             account_id TEXT NOT NULL REFERENCES accounts(id),
             expires_at INTEGER NOT NULL,
             revoked INTEGER NOT NULL DEFAULT 0
         );

         CREATE TABLE IF NOT EXISTS characters (
             id TEXT PRIMARY KEY NOT NULL,
             account_id TEXT NOT NULL UNIQUE REFERENCES accounts(id),
             revision INTEGER NOT NULL,
             spec_json TEXT NOT NULL,
             glb_hash TEXT NOT NULL,
             name TEXT NOT NULL,
             starting_job TEXT NOT NULL,
             px REAL NOT NULL DEFAULT 0.2,
             py REAL NOT NULL DEFAULT 1.65,
             pz REAL NOT NULL DEFAULT 6.4,
             yaw REAL NOT NULL DEFAULT 0,
             hp INTEGER NOT NULL DEFAULT 100,
             xp INTEGER NOT NULL DEFAULT 0,
             level INTEGER NOT NULL DEFAULT 1,
             inventory_json TEXT NOT NULL DEFAULT '[]',
             state_revision INTEGER NOT NULL DEFAULT 0,
             updated_at INTEGER NOT NULL
         );

         CREATE TABLE IF NOT EXISTS avatars (
             hash TEXT PRIMARY KEY NOT NULL,
             bytes INTEGER NOT NULL,
             created_at INTEGER NOT NULL
         );

         CREATE TABLE IF NOT EXISTS game_tickets (
             ticket_hash TEXT PRIMARY KEY NOT NULL,
             account_id TEXT NOT NULL REFERENCES accounts(id),
             character_id TEXT NOT NULL REFERENCES characters(id),
             expires_at INTEGER NOT NULL,
             redeemed_at INTEGER
         );

         CREATE TABLE IF NOT EXISTS reconnect_tokens (
             token_hash TEXT PRIMARY KEY NOT NULL,
             account_id TEXT NOT NULL REFERENCES accounts(id),
             character_id TEXT NOT NULL REFERENCES characters(id),
             expires_at INTEGER NOT NULL,
             consumed_at INTEGER
         );

         CREATE INDEX IF NOT EXISTS idx_accounts_discord ON accounts(discord_id);
         CREATE INDEX IF NOT EXISTS idx_auth_flows_expires ON auth_flows(expires_at);
         CREATE INDEX IF NOT EXISTS idx_game_tickets_expires ON game_tickets(expires_at);
         CREATE INDEX IF NOT EXISTS idx_sessions_account ON sessions(account_id);
         "
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn schema_is_reentrant_and_enforces_one_character_per_account() {
        let db = Connection::open_in_memory().unwrap();
        initialize(&db).unwrap();
        initialize(&db).unwrap();
        db.execute("INSERT INTO accounts(id,discord_id,display_name,created_at) VALUES('a','123','a',0)",[]).unwrap();
        let base = "INSERT INTO characters(id,account_id,revision,spec_json,glb_hash,name,starting_job,updated_at) VALUES(?1,'a',1,'{}','h','Chrona','TimeMage',0)";
        db.execute(base,["c1"]).unwrap();
        assert!(db.execute(base,["c2"]).is_err());
    }
}
