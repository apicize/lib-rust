use crate::{utility::*, ApicizeError, Identifiable};
use reqwest::{ClientBuilder, Identity};
use serde::{Deserialize, Serialize};
use serde_with::base64::{Base64, Standard};
use serde_with::formats::Unpadded;
use serde_with::serde_as;

/// Client certificate used to identify caller
#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum Certificate {
    /// PKCS 12 certificate and and password (.p12 or .pfx)
    #[serde(rename = "PKCS12")]
    PKCS12 {
        /// Uniquely identifies certificate
        #[serde(default = "generate_uuid")]
        id: String,
        /// Human-readable name of certificate
        name: String,
        /// Certificate
        #[serde_as(as = "Base64<Standard, Unpadded>")]
        pfx: Vec<u8>,
        /// Password
        #[serde(skip_serializing_if = "Option::is_none")]
        password: Option<String>,
    },
    /// PEM-encoded certificate and PKCS8 encoded private key files
    #[serde(rename = "PKCS8_PEM")]
    PKCS8PEM {
        /// Uniquely identifies certificate
        #[serde(default = "generate_uuid")]
        id: String,
        /// Human-readable name of certificate
        name: String,
        /// Certificate information
        #[serde_as(as = "Base64<Standard, Unpadded>")]
        pem: Vec<u8>,
        /// Optional key file, if not combining in PKCS8 format
        #[serde_as(as = "Base64<Standard, Unpadded>")]
        key: Vec<u8>,
    },
    /// PEM encoded certificate and key file
    #[serde(rename = "PEM")]
    PEM {
        /// Uniquely identifies certificate
        #[serde(default = "generate_uuid")]
        id: String,
        /// Human-readable name of certificate
        name: String,
        /// Certificate information
        #[serde_as(as = "Base64<Standard, Unpadded>")]
        pem: Vec<u8>,
    },
}

impl Certificate {
    // fn get_id_and_name(&self) -> (&String, &String) {
    //     match self {
    //         Certificate::PKCS8PEM { id, name, .. } => (id, name),
    //         Certificate::PEM { id, name, .. } => (id, name),
    //         Certificate::PKCS12 { id, name, .. } => (id, name),
    //     }
    // }

    /// Append certificate to builder
    pub fn append_to_builder(&self, builder: ClientBuilder) -> Result<ClientBuilder, ApicizeError> {
        let identity_result = match self {
            Certificate::PKCS12 { pfx, password, .. } => Identity::from_pkcs12_der(
                pfx,
                password.clone().unwrap_or(String::from("")).as_str(),
            ),
            Certificate::PKCS8PEM { pem, key, .. } => Identity::from_pkcs8_pem(pem, key),
            Certificate::PEM { pem, .. } => Identity::from_pem(pem),
        };

        match identity_result {
            Ok(identity) => {
                // request_certificate = Some(cert.clone());
                Ok(
                    builder.identity(identity).use_native_tls(), // .tls_info(true)
                )
            }
            Err(err) => Err(ApicizeError::from_reqwest(err)),
        }
    }
}

impl Default for Certificate {
    fn default() -> Self {
        Certificate::PEM {
            id: generate_uuid(),
            name: String::default(),
            pem: Vec::default(),
        }
    }
}

impl Identifiable for Certificate {
    fn get_id(&self) -> &str {
        match self {
            Certificate::PEM { id, .. } => id,
            Certificate::PKCS8PEM { id,  .. } => id,
            Certificate::PKCS12 { id,  .. } => id,
        }
    }

    fn get_name(&self) -> &str {
        match self {
            Certificate::PEM { name, .. } => name,
            Certificate::PKCS8PEM { name,  .. } => name,
            Certificate::PKCS12 { name,  .. } => name,
        }
    }

    fn get_title(&self) -> String {
        let name = self.get_name();
        if name.is_empty() {
            "(Unamed)".to_string()
        } else {
            name.to_string()
        }
    }

    fn clone_as_new(&self, new_name: String) -> Self {
        let mut cloned = self.clone();
        let new_id = generate_uuid();
        
        match cloned {
            Certificate::PEM { ref mut id, ref mut name, ..} => 
                { *id = new_id; *name = new_name; },
            Certificate::PKCS8PEM { ref mut id, ref mut name, ..} => 
                { *id = new_id; *name = new_name; },
            Certificate::PKCS12 { ref mut id, ref mut name, ..} => 
                { *id = new_id; *name = new_name; },
        }

        cloned
    }    
}
