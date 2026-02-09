//! 加密模块
//!
//! 实现 LocalSend 协议的加密功能
//! 用于文件传输的安全验证

use aes_gcm::{Aes256Gcm, Nonce, KeyInit};
use aes_gcm::aead::Aead;
use rand::Rng;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use zeroize::Zeroize;

/// 生成随机密钥
pub fn generate_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    rand::thread_rng().fill(&mut key);
    key
}

/// 生成随机 IV
pub fn generate_iv() -> [u8; 12] {
    let mut iv = [0u8; 12];
    rand::thread_rng().fill(&mut iv);
    iv
}

/// 计算密钥指纹
pub fn compute_fingerprint(key: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key);
    let result = hasher.finalize();
    STANDARD.encode(&result[..16])
}

/// 加密数据
pub fn encrypt(data: &[u8], key: &[u8]) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| e.to_string())?;

    let iv = generate_iv();
    let nonce = Nonce::from_slice(&iv);

    let ciphertext = cipher.encrypt(nonce, data)
        .map_err(|e| e.to_string())?;

    let mut result = iv.to_vec();
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

/// 解密数据
pub fn decrypt(encrypted: &[u8], key: &[u8]) -> Result<Vec<u8>, String> {
    if encrypted.len() < 12 {
        return Err("加密数据太短".to_string());
    }

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| e.to_string())?;

    let iv = &encrypted[..12];
    let ciphertext = &encrypted[12..];

    let nonce = Nonce::from_slice(iv);

    cipher.decrypt(nonce, ciphertext)
        .map_err(|e| e.to_string())
}

/// HMAC 签名
pub fn sign(data: &[u8], key: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(key);
    hasher.update(data);
    hasher.finalize().to_vec()
}

/// 验证 HMAC 签名
pub fn verify(data: &[u8], key: &[u8], signature: &[u8]) -> bool {
    let computed = sign(data, key);
    computed.as_slice() == signature
}

/// 安全地清除密钥
pub fn clear_key(key: &mut [u8]) {
    key.zeroize();
}
