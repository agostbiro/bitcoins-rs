//! Specifies an abstract `AddressEncoder` that is used to convert `Address` to/from the in-memory
//! representation of the associated spending constraint.

use crate::{
    types::tx::{RecipientIdentifier},
};

/// An AddressEncoder encodes and decodes addresses. This struct is used by the Builder to decode
/// addresses, and is associated with a Network object. It handles converting addresses to
/// recipients and vice versa. It also contains a function that wraps a string in the appropriate
/// address type.
///
/// This trait exists to maintain conceptual separation between the user-facing `Address` and the
/// protocol-facing `RecipientIdentifier`.
///
/// A Bitcoin encoder can be found in the `bitcoin` module.
pub trait AddressEncoder {
    /// A type representing the user-facing address, with any disambiguating information.
    type Address;
    /// An error type that will be returned in case of encoding errors
    type Error;
    /// A type representing the in-protocol recipient. This is usually different from the
    /// Address type. The encoder converts between `Strings`, `Address`es, and
    /// `RecipientIdentifier`s
    type RecipientIdentifier: RecipientIdentifier;

    /// Encode a script as an address.
    fn encode_address(s: Self::RecipientIdentifier) -> Result<Self::Address, Self::Error>;

    /// Decode a script from an address.
    fn decode_address(addr: Self::Address) -> Result<Self::RecipientIdentifier, Self::Error>;

    /// Convert a string into an address.
    fn wrap_string(s: String) -> Result<Self::Address, Self::Error>;
}
