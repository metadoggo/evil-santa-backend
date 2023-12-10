use chrono::prelude::*;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct Claims<'a> {
  iss: &'a str,
  // sub: &'a str,
  aud: &'a str,
  iat: usize,
  exp: usize,
  scope: &'a str,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServiceAccount {
  #[serde(rename = "type")]
  pub typ: String,
  pub project_id: String,
  pub private_key_id: String,
  pub private_key: String,
  pub client_email: String,
  pub client_id: String,
  pub auth_uri: String,
  pub token_uri: String,
  pub auth_provider_x509_cert_url: String,
  pub client_x509_cert_url: String,
  pub universe_domain: String,
}

impl ServiceAccount {
  pub fn from_str(s: &str) -> Self {
    serde_json::from_str::<Self>(s).expect("FIREBASE_SERVICE_ACCOUNT should be valid JSON")
  }

  pub fn create_access_token(
    &self,
    expiry: chrono::Duration,
  ) -> Result<String, jsonwebtoken::errors::Error> {
    let iat = Utc::now().timestamp() as usize;
    let exp = Utc::now()
      .checked_add_signed(expiry)
      .expect("valid timestamp")
      .timestamp() as usize;

    let header = Header {
      alg: Algorithm::RS256,
      kid: Some(self.private_key_id.to_string()),
      typ: Some("JWT".to_string()),
      cty: None,
      jku: None,
      jwk: None,
      x5u: None,
      x5c: None,
      x5t: None,
      x5t_s256: None,
    };
    let claims = Claims {
      iss: &self.client_email,
      // sub: &self.client_email,
      aud: &self.token_uri,
      iat,
      exp,
      scope: "https://www.googleapis.com/auth/identitytoolkit",
      // scope: "https://www.googleapis.com/auth/cloud-platform",
    };
    let key = EncodingKey::from_rsa_pem(self.private_key.as_bytes())?;
    encode(&header, &claims, &key)
  }
}
