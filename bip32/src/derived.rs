use crate::{
    curve::model::{ScalarDeserialize, Secp256k1Backend},
    keys::{GenericPrivkey, GenericPubkey},
    model::*,
    path::KeyDerivation,
    xkeys::{hmac_and_split, GenericXPriv, GenericXPub, XKeyInfo, SEED},
    Bip32Error, CURVE_ORDER,
};

// Re-exports
#[cfg(any(feature = "libsecp", feature = "rust-secp"))]
pub use self::keys::{DerivedPrivkey, DerivedPubkey, DerivedXPriv, DerivedXPub};

make_derived_key!(
    /// A Privkey coupled with its derivation
    GenericPrivkey,
    GenericDerivedPrivkey.privkey
);
inherit_has_privkey!(GenericDerivedPrivkey.privkey);
inherit_backend!(GenericDerivedPrivkey.privkey);

impl<'a, T: Secp256k1Backend<'a>> SigningKey<'a, T> for GenericDerivedPrivkey<'a, T> {
    /// The corresponding verifying key
    type VerifyingKey = GenericDerivedPubkey<'a, T>;

    /// Derive the corresponding pubkey
    fn derive_verifying_key(&self) -> Result<Self::VerifyingKey, Bip32Error> {
        Ok(GenericDerivedPubkey {
            pubkey: self.privkey.derive_verifying_key()?,
            derivation: self.derivation.clone(),
        })
    }
}

make_derived_key!(
    /// A Pubkey coupled with its derivation
    GenericPubkey,
    GenericDerivedPubkey.pubkey
);
inherit_has_pubkey!(GenericDerivedPubkey.pubkey);
inherit_backend!(GenericDerivedPubkey.pubkey);

impl<'a, T: Secp256k1Backend<'a>> VerifyingKey<'a, T> for GenericDerivedPubkey<'a, T> {
    type SigningKey = GenericDerivedPrivkey<'a, T>;
}

make_derived_key!(
    /// An XPriv coupled with its derivation
    GenericXPriv,
    GenericDerivedXPriv.xpriv
);
inherit_has_privkey!(GenericDerivedXPriv.xpriv);
inherit_backend!(GenericDerivedXPriv.xpriv);
inherit_has_xkeyinfo!(GenericDerivedXPriv.xpriv);

impl<'a, T: Secp256k1Backend<'a>> GenericDerivedXPriv<'a, T> {
    /// Instantiate a master node using a custom HMAC key.
    pub fn custom_master_node(
        hmac_key: &[u8],
        data: &[u8],
        hint: Option<Hint>,
        backend: &'a T,
    ) -> Result<GenericXPriv<'a, T>, Bip32Error> {
        if data.len() < 16 {
            return Err(Bip32Error::SeedTooShort);
        }
        let parent = KeyFingerprint([0u8; 4]);
        let (key, chain_code) = hmac_and_split(hmac_key, data);
        if key == [0u8; 32] || key > CURVE_ORDER {
            return Err(Bip32Error::InvalidKey);
        }
        let privkey = T::Privkey::from_privkey_array(key)?;
        Ok(GenericXPriv {
            info: XKeyInfo {
                depth: 0,
                parent,
                index: 0,
                chain_code,
                hint: hint.unwrap_or(Hint::SegWit),
            },
            privkey: GenericPrivkey {
                key: privkey,
                backend: Some(backend),
            },
        })
    }

    /// Generate a master node from some seed data. Uses the BIP32-standard hmac key.
    ///
    ///
    /// # Important:
    ///
    /// Use a seed of AT LEAST 128 bits.
    pub fn root_from_seed(
        data: &[u8],
        hint: Option<Hint>,
        backend: &'a T,
    ) -> Result<GenericXPriv<'a, T>, Bip32Error> {
        Self::custom_master_node(SEED, data, hint, backend)
    }

    /// Derive the corresponding xpub
    pub fn to_derived_xpub(&self) -> Result<GenericDerivedXPub<'a, T>, Bip32Error> {
        Ok(GenericDerivedXPub {
            xpub: self.xpriv.derive_verifying_key()?,
            derivation: self.derivation.clone(),
        })
    }

    /// Check if this XPriv is the private ancestor of some other derived key
    pub fn is_private_ancestor_of<D: DerivedKey + HasPubkey<'a, T>>(
        &self,
        other: &D,
    ) -> Result<bool, Bip32Error> {
        if let Some(path) = self.path_to_descendant(other) {
            let descendant = self.derive_private_path(&path)?;
            let descendant_pk_bytes = descendant.derive_pubkey()?;
            Ok(&descendant_pk_bytes == other.pubkey())
        } else {
            Ok(false)
        }
    }
}

impl<'a, T: Secp256k1Backend<'a>> SigningKey<'a, T> for GenericDerivedXPriv<'a, T> {
    /// The corresponding verifying key
    type VerifyingKey = GenericDerivedXPub<'a, T>;

    /// Derive the corresponding pubkey
    fn derive_verifying_key(&self) -> Result<Self::VerifyingKey, Bip32Error> {
        self.to_derived_xpub()
    }
}

impl<'a, T: Secp256k1Backend<'a>> DerivePrivateChild<'a, T> for GenericDerivedXPriv<'a, T> {
    fn derive_private_child(&self, index: u32) -> Result<Self, Bip32Error> {
        Ok(Self {
            xpriv: self.xpriv.derive_private_child(index)?,
            derivation: self.derivation.extended(index),
        })
    }
}

make_derived_key!(
    /// An XPub coupled with its derivation
    GenericXPub,
    GenericDerivedXPub.xpub
);
inherit_has_pubkey!(GenericDerivedXPub.xpub);
inherit_backend!(GenericDerivedXPub.xpub);
inherit_has_xkeyinfo!(GenericDerivedXPub.xpub);

impl<'a, T: Secp256k1Backend<'a>> GenericDerivedXPub<'a, T> {
    /// Derive an XPub from an xpriv
    pub fn from_derived_xpriv(
        xpriv: &GenericDerivedXPriv<'a, T>,
    ) -> Result<GenericDerivedXPub<'a, T>, Bip32Error> {
        xpriv.to_derived_xpub()
    }

    /// Check if this XPriv is the private ancestor of some other derived key
    pub fn is_public_ancestor_of<D: DerivedKey + HasPubkey<'a, T>>(
        &self,
        other: &D,
    ) -> Result<bool, Bip32Error> {
        if let Some(path) = self.path_to_descendant(other) {
            let descendant = self.derive_public_path(&path)?;
            Ok(descendant.pubkey() == other.pubkey())
        } else {
            Ok(false)
        }
    }
}

impl<'a, T: Secp256k1Backend<'a>> VerifyingKey<'a, T> for GenericDerivedXPub<'a, T> {
    type SigningKey = GenericDerivedXPriv<'a, T>;
}

impl<'a, T: Secp256k1Backend<'a>> DerivePublicChild<'a, T> for GenericDerivedXPub<'a, T> {
    fn derive_public_child(&self, index: u32) -> Result<Self, Bip32Error> {
        Ok(Self {
            xpub: self.xpub.derive_public_child(index)?,
            derivation: self.derivation.extended(index),
        })
    }
}

#[cfg(any(feature = "libsecp", feature = "rust-secp"))]
#[doc(hidden)]
pub mod keys {
    use super::*;

    use crate::Secp256k1;

    /// A Privkey coupled with its (purported) derivation path
    pub type DerivedPrivkey<'a> = GenericDerivedPrivkey<'a, Secp256k1<'a>>;

    /// A Pubkey coupled with its (purported) derivation path
    pub type DerivedPubkey<'a> = GenericDerivedPubkey<'a, Secp256k1<'a>>;

    /// An XPriv coupled with its (purported) derivation path
    pub type DerivedXPriv<'a> = GenericDerivedXPriv<'a, Secp256k1<'a>>;

    /// An XPub coupled with its (purported) derivation path
    pub type DerivedXPub<'a> = GenericDerivedXPub<'a, Secp256k1<'a>>;
}
