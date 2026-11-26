//! Secure local storage using Windows DPAPI (CryptProtectData / CryptUnprotectData)
//! with application-specific entropy so other programs or users cannot read saved holdings.

use std::fs;
use std::path::Path;

const MAGIC_HEADER: &[u8; 5] = b"SSTK\x01";
const APP_ENTROPY: &[u8] = b"simple_stock::tgsastudio::dpapi::v1::#9F4A7C2E";

#[cfg(target_os = "windows")]
mod dpapi {
    use super::APP_ENTROPY;
    use std::ptr;

    #[repr(C)]
    struct DataBlob {
        cb_data: u32,
        pb_data: *mut u8,
    }

    const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x01;

    #[link(name = "crypt32")]
    unsafe extern "system" {
        fn CryptProtectData(
            p_data_in: *const DataBlob,
            sz_data_descr: *const u16,
            p_optional_entropy: *const DataBlob,
            pv_reserved: *mut core::ffi::c_void,
            p_prompt_struct: *const core::ffi::c_void,
            dw_flags: u32,
            p_data_out: *mut DataBlob,
        ) -> i32;

        fn CryptUnprotectData(
            p_data_in: *const DataBlob,
            ppsz_data_descr: *mut *mut u16,
            p_optional_entropy: *const DataBlob,
            pv_reserved: *mut core::ffi::c_void,
            p_prompt_struct: *const core::ffi::c_void,
            dw_flags: u32,
            p_data_out: *mut DataBlob,
        ) -> i32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LocalFree(h_mem: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
    }

    pub fn encrypt(plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let in_blob = DataBlob {
            cb_data: plaintext.len() as u32,
            pb_data: plaintext.as_ptr() as *mut u8,
        };
        let entropy_blob = DataBlob {
            cb_data: APP_ENTROPY.len() as u32,
            pb_data: APP_ENTROPY.as_ptr() as *mut u8,
        };
        let mut out_blob = DataBlob {
            cb_data: 0,
            pb_data: ptr::null_mut(),
        };

        let ok = unsafe {
            CryptProtectData(
                &in_blob,
                ptr::null(),
                &entropy_blob,
                ptr::null_mut(),
                ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out_blob,
            )
        };

        if ok == 0 {
            return Err("DPAPI CryptProtectData failed".to_string());
        }

        let result = unsafe {
            let slice = std::slice::from_raw_parts(out_blob.pb_data, out_blob.cb_data as usize);
            let vec = slice.to_vec();
            LocalFree(out_blob.pb_data as *mut core::ffi::c_void);
            vec
        };
        Ok(result)
    }

    pub fn decrypt(ciphertext: &[u8]) -> Result<Vec<u8>, String> {
        let in_blob = DataBlob {
            cb_data: ciphertext.len() as u32,
            pb_data: ciphertext.as_ptr() as *mut u8,
        };
        let entropy_blob = DataBlob {
            cb_data: APP_ENTROPY.len() as u32,
            pb_data: APP_ENTROPY.as_ptr() as *mut u8,
        };
        let mut out_blob = DataBlob {
            cb_data: 0,
            pb_data: ptr::null_mut(),
        };

        let ok = unsafe {
            CryptUnprotectData(
                &in_blob,
                ptr::null_mut(),
                &entropy_blob,
                ptr::null_mut(),
                ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out_blob,
            )
        };

        if ok == 0 {
            return Err("DPAPI CryptUnprotectData failed".to_string());
        }

        let result = unsafe {
            let slice = std::slice::from_raw_parts(out_blob.pb_data, out_blob.cb_data as usize);
            let vec = slice.to_vec();
            LocalFree(out_blob.pb_data as *mut core::ffi::c_void);
            vec
        };
        Ok(result)
    }
}

pub fn encrypt_bytes(plaintext: &[u8]) -> Result<Vec<u8>, String> {
    #[cfg(target_os = "windows")]
    {
        let mut out = Vec::with_capacity(MAGIC_HEADER.len() + plaintext.len() + 256);
        out.extend_from_slice(MAGIC_HEADER);
        out.extend_from_slice(&dpapi::encrypt(plaintext)?);
        Ok(out)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(plaintext.to_vec())
    }
}

pub fn decrypt_bytes(data: &[u8]) -> Result<Vec<u8>, String> {
    #[cfg(target_os = "windows")]
    {
        if data.starts_with(MAGIC_HEADER) {
            dpapi::decrypt(&data[MAGIC_HEADER.len()..])
        } else {
            Err("Invalid encrypted file header".to_string())
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(data.to_vec())
    }
}

pub fn write_encrypted(path: &Path, plaintext: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let encrypted = encrypt_bytes(plaintext)?;
    fs::write(path, encrypted).map_err(|e| format!("Write failed: {e}"))
}

pub fn read_encrypted(path: &Path) -> Result<Vec<u8>, String> {
    let raw = fs::read(path).map_err(|e| format!("Read failed: {e}"))?;
    decrypt_bytes(&raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dpapi_roundtrip_and_opacity() {
        let secret = br#"{"symbol":"2330","shares":1000,"avg_cost":950.5}"#;
        let encrypted = encrypt_bytes(secret).expect("DPAPI encrypt failed");
        assert!(encrypted.starts_with(MAGIC_HEADER));
        // Ensure plaintext symbols/numbers do not leak into ciphertext
        let enc_lossy = String::from_utf8_lossy(&encrypted);
        assert!(!enc_lossy.contains("2330"));
        assert!(!enc_lossy.contains("avg_cost"));

        let decrypted = decrypt_bytes(&encrypted).expect("DPAPI decrypt failed");
        assert_eq!(decrypted, secret);
    }
}
