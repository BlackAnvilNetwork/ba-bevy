//! Download and cache approved character models from Black Anvil's gateway.
//!
//! The zone sends only immutable SHA-256 identifiers. Never load a filesystem
//! path, URL or glTF external buffer supplied by another player. This module
//! only forms a URL from an allowlisted gateway origin and verified hash.

use std::collections::HashMap;
use std::fs;
use std::io::{Read,Write};
use std::path::{Path,PathBuf};
use std::sync::{Mutex,OnceLock};
use std::thread;
use std::time::{Duration,Instant};

use sha2::{Digest,Sha256};

const MAX_AVATAR_BYTES: u64=32*1024*1024;
static ATTEMPTS: OnceLock<Mutex<HashMap<String,Instant>>>=OnceLock::new();

fn valid_hash(hash:&str)->bool {
    hash.len()==64 && hash.bytes().all(|b| b.is_ascii_hexdigit())
}

fn cache_root()->Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("BA_CHARACTER_ROOT")?).join("cache"))
}

fn safe_gateway_url()->Option<String> {
    let base=std::env::var("BA_GATEWAY_PUBLIC_URL").ok()?;
    let url=reqwest::Url::parse(&base).ok()?;
    if url.scheme()!="https" && !(url.scheme()=="http" &&
       url.host_str().is_some_and(|h| h=="127.0.0.1" || h=="localhost" || h=="::1")) {
        return None;
    }
    if url.query().is_some() || url.fragment().is_some() || url.username()!="" || url.password().is_some() {
        return None;
    }
    Some(base.trim_end_matches('/').to_owned())
}

fn glb_bytes_valid(data:&[u8])->bool {
    data.len()>=20 && data.len() as u64<=MAX_AVATAR_BYTES && &data[..4]==b"glTF" &&
    u32::from_le_bytes(data[4..8].try_into().unwrap())==2 &&
    u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize==data.len()
}

fn verify(hash:&str,bytes:&[u8])->bool {
    valid_hash(hash) && glb_bytes_valid(bytes) &&
    format!("{:x}",Sha256::digest(bytes)).eq_ignore_ascii_case(hash)
}

fn file_is_verified(path:&Path,hash:&str)->bool {
    fs::metadata(path).ok().is_some_and(|m|m.len()<=MAX_AVATAR_BYTES) &&
        fs::read(path).ok().is_some_and(|v|verify(hash,&v))
}

fn download_avatar(base:&str,root:&Path,hash:&str)->Result<(),String> {
    fs::create_dir_all(root).map_err(|_|"cannot_create_cache")?;
    let dest=root.join(format!("{hash}.glb"));
    if file_is_verified(&dest,hash) {return Ok(());}
    let client=reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .build().map_err(|_|"download_client_error")?;
    let url=format!("{base}/v1/assets/{hash}");
    let mut res=client.get(url).send().map_err(|_|"asset_gateway_unavailable")?;
    if !res.status().is_success() {return Err("asset_not_approved_or_missing".into());}
    if res.content_length().is_some_and(|n|n>MAX_AVATAR_BYTES) {
        return Err("remote_asset_too_large".into());
    }
    let mut bytes=Vec::new();
    res.take(MAX_AVATAR_BYTES+1).read_to_end(&mut bytes)
        .map_err(|_|"avatar_download_failed")?;
    if !verify(hash,&bytes) {return Err("avatar_hash_or_glb_mismatch".into());}

    let temp=root.join(format!(".download-{hash}-{}",std::process::id()));
    let write=(||{
        let mut file=fs::File::create(&temp).map_err(|_|"cannot_write_avatar_cache")?;
        file.write_all(&bytes).map_err(|_|"cannot_write_avatar_cache")?;
        file.sync_all().map_err(|_|"cannot_sync_avatar_cache")?;
        fs::rename(&temp,&dest).map_err(|_|"cannot_publish_avatar_cache")?;
        Ok::<(),&str>(())
    })();
    if write.is_err(){let _=fs::remove_file(&temp);}
    write.map_err(str::to_owned)
}

/// Nonblocking and rate-limited per content hash. Safe to call on every new
/// snapshot. Failed downloads are retried after 30 seconds, not every frame.
pub fn prefetch(hash:&str) {
    if !valid_hash(hash){return;}
    let Some(root)=cache_root() else{return};
    if file_is_verified(&root.join(format!("{hash}.glb")),hash){return;}
    let Some(base)=safe_gateway_url() else{return};
    let attempts=ATTEMPTS.get_or_init(||Mutex::new(HashMap::new()));
    {
        let Ok(mut map)=attempts.lock() else{return};
        if map.get(hash).is_some_and(|last|last.elapsed()<Duration::from_secs(30)){return;}
        if map.len()>512 {
            map.retain(|_,time|time.elapsed()<Duration::from_secs(60));
        }
        map.insert(hash.to_string(),Instant::now());
    }
    let hash=hash.to_string();
    let _=thread::Builder::new().name("ba-avatar-fetch".into()).spawn(move||{
        if let Err(reason)=download_avatar(&base,&root,&hash) {
            eprintln!("Avatar {hash} unavailable: {reason}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_sha256_content_ids_are_allowed() {
        assert!(valid_hash(&"a".repeat(64)));
        assert!(!valid_hash("../other/file.glb"));
        assert!(!valid_hash(&"g".repeat(64)));
    }
    #[test]
    fn incorrect_hash_never_passes_cache_verification() {
        let mut data=b"glTF".to_vec();
        data.extend_from_slice(&2_u32.to_le_bytes());
        data.extend_from_slice(&20_u32.to_le_bytes());
        data.extend_from_slice(&[0_u8;8]);
        assert!(!verify(&"a".repeat(64),&data));
        let actual=format!("{:x}",Sha256::digest(&data));
        assert!(verify(&actual,&data));
    }
}
