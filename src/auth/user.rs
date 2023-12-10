use anyhow::{anyhow, bail, Result};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest::StatusCode;
use serde_with::skip_serializing_none;
use std::fmt::Debug;
use std::ops::Add;
use std::time::Duration;
use std::{collections::HashMap, time::SystemTime};

use serde::{Deserialize, Serialize};
use serde_with::{json::JsonString, serde_as};

use super::{CustomClaims, ServiceAccount, User};

#[serde_as]
#[allow(non_snake_case)]
#[derive(Debug, Serialize)]
struct SetCustomAttributesPayload<'a> {
  localId: &'a str,
  #[serde_as(as = "JsonString")]
  customAttributes: CustomClaims,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize)]
struct FederatedUserIdentifier<'a> {
  providerId: &'a str,
  rawId: &'a str,
}

#[allow(non_snake_case)]
#[skip_serializing_none]
#[derive(Debug, Serialize)]
struct AccountsLookupPayload<'a> {
  idToken: Option<&'a str>,
  localId: Option<Vec<&'a str>>,
  email: Option<Vec<&'a str>>,
  delegatedProjectNumber: Option<&'a str>,
  phoneNumber: Option<Vec<&'a str>>,
  federatedUserId: Option<Vec<FederatedUserIdentifier<'a>>>,
  tenantId: Option<&'a str>,
  targetProjectId: Option<&'a str>,
  initialEmail: Option<Vec<&'a str>>,
}

#[derive(Debug, Deserialize)]
pub struct GetAccountInfoResponse {
  pub kind: String,
  pub users: Vec<User>,
}

#[derive(Debug, Clone)]
pub struct UserService {
  sa: ServiceAccount,
  update_url: String,
  lookup_url: String,
  http_client: reqwest::Client,
  auth_header: String,
  id_token_expiry: SystemTime,
}

#[derive(Debug, Deserialize, Clone)]
struct IdToken {
  pub access_token: String,
  pub token_type: String,
  pub expires_in: u64,
}

impl UserService {
  pub fn new(api_key: &str, sa: ServiceAccount) -> Self {
    Self {
      sa,
      update_url: format!(
        "https://identitytoolkit.googleapis.com/v1/accounts:update?key={}",
        api_key
      ),
      lookup_url: format!(
        "https://identitytoolkit.googleapis.com/v1/accounts:lookup?key={}",
        api_key
      ),
      http_client: reqwest::Client::new(),
      auth_header: String::from(""),
      id_token_expiry: SystemTime::now(),
    }
  }

  async fn fetch_id_token(&self) -> Result<IdToken> {
    let jwt = self
      .sa
      .create_access_token(chrono::Duration::minutes(5))
      .map_err(|err| anyhow!(err))?;

    let mut request_token_form = HashMap::new();
    request_token_form.insert("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer");
    request_token_form.insert("assertion", &jwt);
    let res = self
      .http_client
      .post(&self.sa.token_uri)
      .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
      .form(&request_token_form)
      .send()
      .await?;

    match res.status() {
      StatusCode::OK => res.json().await.map_err(|err| anyhow!(err)),
      status => bail!("{} {}", status, res.text().await?),
    }
  }

  async fn get_auth_header(&mut self) -> Result<String> {
    let now = SystemTime::now();
    if self.id_token_expiry < now || self.auth_header.is_empty() {
      let id_token = Self::fetch_id_token(self).await?;
      self.auth_header = format!("{} {}", &id_token.token_type, &id_token.access_token);
      self.id_token_expiry = now.add(Duration::from_secs(id_token.expires_in));
    }
    Ok(self.auth_header.clone())
  }

  pub async fn set_custom_attributes(&mut self, uid: &str, attr: CustomClaims) -> Result<()> {
    self.get_auth_header().await?;
    let res = self
      .http_client
      .post(&self.update_url)
      .header(AUTHORIZATION, &self.auth_header)
      .header(CONTENT_TYPE, "application/json")
      .json(&SetCustomAttributesPayload {
        localId: uid,
        customAttributes: attr,
      })
      .send()
      .await?;

    match res.status() {
      StatusCode::OK => Ok(()),
      status => bail!("{} {}", status, res.text().await?),
    }
  }

  pub async fn lookup(&mut self, uid: &str) -> Result<User> {
    self.get_auth_header().await?;
    let res = self
      .http_client
      .post(&self.lookup_url)
      .header(AUTHORIZATION, &self.auth_header)
      .json(&AccountsLookupPayload {
        idToken: None,
        localId: Some(vec![uid]),
        email: None,
        delegatedProjectNumber: None,
        phoneNumber: None,
        federatedUserId: None,
        tenantId: None,
        targetProjectId: None,
        initialEmail: None,
      })
      .send()
      .await?;

    match res.status() {
      StatusCode::OK => res
        .json::<GetAccountInfoResponse>()
        .await
        .map_err(|err| anyhow!(err))?
        .users
        .into_iter()
        .nth(0)
        .ok_or(anyhow!("Not found")),
      status => bail!("{} {}", status, res.text().await?),
    }
  }
}
