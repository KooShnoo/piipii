use crate::pp::{ReadSDPiiPersonalData, SDPiiPersonalData, WriteSDPiiPersonalData};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use sha1::{Digest, Sha1};
use std::io::Cursor;

/// size (in bytes) of a `savedata.bin` file
pub const SAVEDATA_SIZE: usize = 0x34000;
/// size (in bytes) of a decryption chunk in a `savedata.bin` file
const CHUNK_SIZE: usize = 0x10;
/// amount of decryption chunks in a `savedata.bin` file
const CHUNK_COUNT: usize = SAVEDATA_SIZE / CHUNK_SIZE;

/// offset (in bytes) into a `savedata.bin` file where the PiiBox data for save slot 1 is stored.
const PIIBOX_SAVEDATA_OFFSET: usize = 0x1360;
const SIZEOF_SDPPD: usize = 0x2D;

pub fn extract_piibox(savedata: &[u8]) -> Box<[SDPiiPersonalData]> {
    let mut cursor = Cursor::new(savedata);
    cursor.set_position(PIIBOX_SAVEDATA_OFFSET as u64);

    let pii_box_len = cursor.read_u16::<BigEndian>().unwrap();
    (0..pii_box_len)
        .map(|_| cursor.read_sd_ppd().unwrap())
        .collect::<Box<_>>()
}

pub fn write_piibox(savedata: &mut [u8], pii_box: &[SDPiiPersonalData]) {
    let mut cursor = Cursor::new(savedata);
    cursor.set_position(PIIBOX_SAVEDATA_OFFSET as u64);
    cursor.write_u16::<BigEndian>(pii_box.len().try_into().unwrap());

    for pii in pii_box {
        cursor.write_sd_ppd(pii);
    }
}

/// See <https://gist.github.com/Lincoln-LM/a12b747d8595f523607a7bae0b7936f0>
pub fn decrypt_savedata(savedata: &mut [u8]) {
    for chunk_idx in (1..CHUNK_COUNT).rev() {
        // the chunk's offset in `savedata.bin`
        let chunk_pos: usize = chunk_idx * CHUNK_SIZE;
        for i in 0..CHUNK_SIZE {
            let index = chunk_pos + i;
            let offset = chunk_pos + ((chunk_idx + i) & 0xF) - CHUNK_SIZE;
            savedata[index] = savedata[index].wrapping_sub(savedata[offset]);
        }
    }

    let mut hasher = Sha1::new();
    hasher.update(&savedata[20..]);
    let result = hasher.finalize();
    assert_eq!(result[..], savedata[..20])
}

/// See <https://gist.github.com/Lincoln-LM/a12b747d8595f523607a7bae0b7936f0>
pub fn encrypt_savedata(savedata: &mut [u8]) {
    let mut hasher = Sha1::new();
    hasher.update(&savedata[20..]);
    let result = hasher.finalize();
    assert_eq!(result.len(), 20);
    savedata[..20].copy_from_slice(&result);

    for chunk_idx in (1..CHUNK_COUNT) {
        // the chunk's offset in `savedata.bin`
        let chunk_pos: usize = chunk_idx * CHUNK_SIZE;
        for i in 0..CHUNK_SIZE {
            let index = chunk_pos + i;
            let offset = chunk_pos + ((chunk_idx + i) & 0xF) - CHUNK_SIZE;
            savedata[index] = savedata[index].wrapping_add(savedata[offset]);
        }
    }
}
