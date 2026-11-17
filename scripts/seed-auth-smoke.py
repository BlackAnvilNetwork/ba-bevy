"""Seed ONLY an isolated CI smoke database. Not a production endpoint."""
import hashlib, json, os, secrets, sqlite3, sys, uuid
db_path, output = sys.argv[1:]
db = sqlite3.connect(db_path)
result = {}
for suffix in ("a", "b"):
    account_id = str(uuid.uuid4())
    session = secrets.token_urlsafe(32)
    db.execute(
        "INSERT INTO accounts(id,discord_id,display_name,created_at) VALUES(?,?,?,?)",
        (account_id, "99999999999999999" + ("1" if suffix=="a" else "2"), "Smoke Player " + suffix, 0),
    )
    db.execute(
        "INSERT INTO sessions(token_hash,account_id,expires_at) VALUES(?,?,?)",
        (hashlib.sha256(session.encode()).hexdigest(), account_id, 4_100_000_000),
    )
    result[suffix] = {"accountId": account_id, "sessionToken": session}
db.commit()
with open(output, "w", encoding="utf-8") as f:
    json.dump(result, f)
os.chmod(output, 0o600)
print("Seeded two ephemeral test-only Discord identities in isolated SQLite database.")
