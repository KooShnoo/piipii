use std::{
    error::Error,
    io::{Cursor, Write},
    println,
};

use crate::{
    dex::{
        PiiSpecies, ARCEUS_FORMS, BURMY_FORMS, CASTFORM_FORMS, CHERRIM_FORMS, DEOXYS_FORMS,
        GASTRODON_FORMS, GIRATINA_FORMS, MOVES, POKEMON_NAMES, ROTOM_FORMS, SHAYMIN_FORMS,
        SHELLOS_FORMS, TRAITS, UNOWN_FORMS, WORMADAM_FORMS,
    },
    LOCALE,
};
use bitfield::bitfield;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use num_enum::{FromPrimitive, IntoPrimitive, TryFromPrimitive, TryFromPrimitiveError};
use sha1::{Digest, Sha1};
use web_sys::{
    js_sys::{self, Date, Reflect},
    wasm_bindgen::JsValue,
};

bitfield! {
    /// The first 2 bytes of [SDPiiPersonalData]. These bytes are packed, so this separate bitfield struct is used.
    pub struct SDPiiPersonalDataPacked(u16);
    impl Debug;
    impl new;
    u16;
    pub mons_no, set_mons_no: 15, 7;
    pub form_no, set_form_no: 6, 2;
    pub sex, set_sex: 2, 0;

}

#[repr(u16)]
#[derive(TryFromPrimitive, IntoPrimitive)]
pub enum PiiSex {
    Male = 0,
    Female = 1,
    Unknown = 2,
}

impl std::fmt::Display for PiiSex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Male => f.write_str("Male"),
            Self::Female => f.write_str("Female"),
            Self::Unknown => f.write_str("Unknown"),
        }
    }
}

/// `SD_PiiPersonalData`. SD is SaveData, Pii is Wii Pokémon,
/// and Personal Data refers to data specific to one pokémon entity.
/// The name comes from RTTI in Pokémon Rumble's executable.
#[derive(Clone, Debug, PartialEq)]
pub struct SDPiiPersonalData {
    /// National dex number.
    pub mons_no: u16,
    pub form_no: u16,
    pub sex: u16,
    pub move1_id: u16,
    pub move2_id: u16,
    pub level: u16,
    pub bonus_max_hp: u32,
    pub bonus_attack_power: u32,
    pub bonus_defence_power: u32,
    pub bonus_speed: u32,
    /// Called `prefix` in the game's code.
    pub trait_: u16,
    /// Bitflags. Currently undocumented.
    pub flags: u16,
    /// Randomized value used for determining shinies (and probably other things too).
    pub pii_id: u32,
    /// Probably used for randomization.
    pub time: u64,
    /// Called `oya_id` in the game's code.
    pub trainer_id: u32,
}

impl SDPiiPersonalData {
    pub fn move_name(&self, move_no: u8) -> Option<&'static str> {
        let move_id: usize = match move_no {
            1 => self.move1_id.into(),
            2 => self.move2_id.into(),
            _ => return None,
        };

        if move_id == 0 {
            None
        } else {
            Some(MOVES[move_id])
        }
    }

    pub fn name(&self) -> &str {
        self.name_and_poke_api_sprite_id().0
    }

    pub fn trait_(&self) -> Option<&'static Trait> {
        if self.trait_ == 0 {
            return None;
        }
        TRAITS.get(self.trait_ as usize - 1)
    }

    /// Accounts for a Pii's alternate forms.
    pub fn name_and_poke_api_sprite_id(&self) -> (&str, &str) {
        // TODO: handle invalid species
        let species = PiiSpecies::try_from(self.mons_no).unwrap();
        match species {
            PiiSpecies::UNOWN => UNOWN_FORMS[self.form_no as usize],
            PiiSpecies::CASTFORM => CASTFORM_FORMS[self.form_no as usize],
            PiiSpecies::DEOXYS => DEOXYS_FORMS[self.form_no as usize],
            PiiSpecies::BURMY => BURMY_FORMS[self.form_no as usize],
            PiiSpecies::WORMADAM => WORMADAM_FORMS[self.form_no as usize],
            PiiSpecies::CHERRIM => CHERRIM_FORMS[self.form_no as usize],
            PiiSpecies::SHELLOS => SHELLOS_FORMS[self.form_no as usize],
            PiiSpecies::GASTRODON => GASTRODON_FORMS[self.form_no as usize],
            PiiSpecies::ROTOM => ROTOM_FORMS[self.form_no as usize],
            PiiSpecies::GIRATINA => GIRATINA_FORMS[self.form_no as usize],
            PiiSpecies::SHAYMIN => SHAYMIN_FORMS[self.form_no as usize],
            PiiSpecies::ARCEUS => ARCEUS_FORMS[self.form_no as usize],
            _ => (
                POKEMON_NAMES[self.mons_no as usize - 1],
                &*self.mons_no.to_string().leak(),
            ),
        }
    }

    pub fn unix_time(&self) -> String {
        const OS_TIME_SPEED: f64 = 243_000_000.0 / 4.0; // 60_750_000.0

        const UNIX_EPOCH_OFFSET: f64 = 946_684_800.0;

        let unix_secs = (self.time as f64) / OS_TIME_SPEED + UNIX_EPOCH_OFFSET;
        let unix_ms = unix_secs * 1000_f64;
        // let unix_ms= unix_secs ;
        let d = js_sys::Date::new(&unix_ms.into());

        d.to_locale_string(&LOCALE, &JsValue::undefined()).into()
    }

    pub fn is_shiny(&self) -> bool {
        let trainer_id_high = self.trainer_id >> 16;
        let trainer_id_low = self.trainer_id & 0xFFFF;

        let pii_id_high = self.pii_id >> 16;
        let pii_id_low = self.pii_id & 0xFFFF;

        let xor_result = trainer_id_high ^ trainer_id_low ^ pii_id_high ^ pii_id_low;

        xor_result < 8
    }

    pub fn sex(&self) -> Result<PiiSex, TryFromPrimitiveError<PiiSex>> {
        let raw_sex = self.sex;
        PiiSex::try_from(raw_sex)
    }

    pub fn set_species(&mut self, species: PiiSpecies) {
        self.mons_no = species.into();
    }

    pub fn sprite_src(&self) -> String {
        let poke_api_sprite_id = self.name_and_poke_api_sprite_id().1;
        format!(
            "https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/other/home/{shiny_path}{poke_api_sprite_id}.png",
            shiny_path = if self.is_shiny() {"shiny/"} else {""}
        )
    }
}

/// A Pii's special trait. See <https://bulbapedia.bulbagarden.net/wiki/Special_Traits>
#[derive(Debug)]
pub struct Trait {
    pub name: &'static str,
    pub description: &'static str,
}

/// Wrapper type extending [u32]. Adds support for splitting/joining two [u16]s holding the highest 16 bits and the lowest 16 bits.
#[derive(Debug)]
struct U32(u32);
impl From<U32> for u32 {
    fn from(val: U32) -> Self {
        val.0
    }
}
impl From<(u16, u16)> for U32 {
    fn from(values: (u16, u16)) -> Self {
        let (high, low) = values;
        let (high, low) = (u32::from(high), u32::from(low));
        let packed = (high << 16) | low;
        U32(packed)
    }
}
impl From<U32> for (u16, u16) {
    fn from(x: U32) -> Self {
        let high = (x.0 >> 16) as u16;
        let low = x.0 as u16;
        (high, low)
    }
}

/// Extends [ReadBytesExt] with methods for reading [SDPiiPersonalData].
pub trait ReadSDPiiPersonalData: ReadBytesExt {
    fn read_sd_ppd(&mut self) -> Result<SDPiiPersonalData, Box<dyn Error>> {
        let packed_mons_no_form_no = self.read_u16::<BigEndian>()?;
        let move1_id = self.read_u16::<BigEndian>()?;
        let move2_id = self.read_u16::<BigEndian>()?;
        let level = self.read_u16::<BigEndian>()?;
        let lo_bonus_max_hp = self.read_u16::<BigEndian>()?;
        let lo_bonus_attack_power = self.read_u16::<BigEndian>()?;
        let lo_bonus_defence_power = self.read_u16::<BigEndian>()?;
        let lo_bonus_speed = self.read_u16::<BigEndian>()?;
        let trait_ = self.read_u16::<BigEndian>()?;
        let flags = self.read_u16::<BigEndian>()?;
        let pii_id = self.read_u32::<BigEndian>()?;
        let hi_bonus_max_hp = self.read_u16::<BigEndian>()?;
        let hi_bonus_attack_power = self.read_u16::<BigEndian>()?;
        let hi_bonus_defence_power = self.read_u16::<BigEndian>()?;
        let hi_bonus_speed = self.read_u16::<BigEndian>()?;
        let time = self.read_u64::<BigEndian>()?;
        let trainer_id = self.read_u32::<BigEndian>()?;
        // idk what this is
        assert_eq!(self.read_u8()?, 0);

        let packed = SDPiiPersonalDataPacked(packed_mons_no_form_no);

        Ok(SDPiiPersonalData {
            mons_no: packed.mons_no(),
            form_no: packed.form_no(),
            sex: packed.sex(),
            move1_id,
            move2_id,
            level,
            bonus_max_hp: U32::from((hi_bonus_max_hp, lo_bonus_max_hp)).into(),
            bonus_attack_power: U32::from((hi_bonus_attack_power, lo_bonus_attack_power)).into(),
            bonus_defence_power: U32::from((hi_bonus_defence_power, lo_bonus_defence_power)).into(),
            bonus_speed: U32::from((hi_bonus_speed, lo_bonus_speed)).into(),
            trait_,
            flags,
            pii_id,
            time,
            trainer_id,
        })
    }
}
impl<R: std::io::Read + ?Sized> ReadSDPiiPersonalData for R {}

/// Extends [WriteBytesExt] with methods for writing [SDPiiPersonalData].
pub trait WriteSDPiiPersonalData: WriteBytesExt {
    fn write_sd_ppd(&mut self, ppd: &SDPiiPersonalData) -> Result<(), Box<dyn Error>> {
        let packed_mons_no_form_no =
            SDPiiPersonalDataPacked::new(ppd.mons_no, ppd.form_no, ppd.sex);

        let (hi_bonus_max_hp, lo_bonus_max_hp) = <(u16, u16)>::from(U32(ppd.bonus_max_hp));
        let (hi_bonus_attack_power, lo_bonus_attack_power) =
            <(u16, u16)>::from(U32(ppd.bonus_attack_power));
        let (hi_bonus_defence_power, lo_bonus_defence_power) =
            <(u16, u16)>::from(U32(ppd.bonus_defence_power));
        let (hi_bonus_speed, lo_bonus_speed) = <(u16, u16)>::from(U32(ppd.bonus_speed));

        self.write_u16::<BigEndian>(packed_mons_no_form_no.0);
        self.write_u16::<BigEndian>(ppd.move1_id)?;
        self.write_u16::<BigEndian>(ppd.move2_id)?;
        self.write_u16::<BigEndian>(ppd.level)?;
        self.write_u16::<BigEndian>(lo_bonus_max_hp)?;
        self.write_u16::<BigEndian>(lo_bonus_attack_power)?;
        self.write_u16::<BigEndian>(lo_bonus_defence_power)?;
        self.write_u16::<BigEndian>(lo_bonus_speed)?;
        self.write_u16::<BigEndian>(ppd.trait_)?;
        self.write_u16::<BigEndian>(ppd.flags)?;
        self.write_u32::<BigEndian>(ppd.pii_id)?;
        self.write_u16::<BigEndian>(hi_bonus_max_hp)?;
        self.write_u16::<BigEndian>(hi_bonus_attack_power)?;
        self.write_u16::<BigEndian>(hi_bonus_defence_power)?;
        self.write_u16::<BigEndian>(hi_bonus_speed)?;
        self.write_u64::<BigEndian>(ppd.time)?;
        self.write_u32::<BigEndian>(ppd.trainer_id)?;
        self.write_u8(0);

        Ok(())
    }
}
impl<R: std::io::Write + ?Sized> WriteSDPiiPersonalData for R {}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        dex::PiiSpecies,
        save_data::{decrypt_savedata, encrypt_savedata, extract_piibox, write_piibox},
    };
    use byteorder::WriteBytesExt;
    use core::assert_eq;
    use std::{fs, io::Write};

    #[test]
    fn parse_savedata() {
        // put any valid savedata to test
        let mut savedata: [u8; include_bytes!("../savedata.bin").len()] =
            *include_bytes!("../savedata.bin");
        let mut unaltered_savedata = savedata.clone();

        decrypt_savedata(&mut savedata);
        let mut pii_box = extract_piibox(&savedata).into_vec();

        write_piibox(&mut savedata, &pii_box);
        encrypt_savedata(&mut savedata);
    }

    #[test]
    fn u32_from_high_and_low() {
        assert_eq!(U32::from((0xaaaa, 0xbbbb)).0, 0xaaaabbbb);
    }
    #[test]
    fn u32_to_high_and_low() {
        assert_eq!(<(u16, u16)>::from(U32(0xaaaabbbb)), (0xaaaa, 0xbbbb));
    }
}
