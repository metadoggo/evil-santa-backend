pub mod firebase;
pub mod user;

use std::collections::HashMap;

use chrono::{DateTime, Utc};
pub use firebase::ServiceAccount;

use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use uuid::Uuid;

use crate::api::games::{PLAY_PERMISSION, VIEW_PERMISSION, OWNER_PERMISSION};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CustomClaims {
  #[serde(rename = "g")]
  pub games: HashMap<String, i64>,
}

// impl<'de> Visitor<'de> for CustomClaims {
//   type Value = bool;

//   fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
//       formatter.write_str("a json string containing custom claims")
//   }

//   fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
//       where E: de::Error
//   {
//       bool::from_str(s).map_err(de::Error::custom)
//   }
// }


/// The Jwt claims decoded from the user token. Can also be viewed as the Firebase User
/// information.
#[derive(Deserialize, Clone)]
pub struct MyFirebaseUser {
  pub provider_id: Option<String>,
  pub name: Option<String>,
  pub picture: Option<String>,
  pub iss: String,
  pub aud: String,
  pub auth_time: u64,
  pub user_id: String,
  pub sub: String,
  pub iat: u64,
  pub exp: u64,
  pub email: Option<String>,
  pub email_verified: Option<bool>,
  #[serde(rename = "g", default)]
  pub games: HashMap<String, i64>,
}

impl MyFirebaseUser {
  pub fn can_edit(&self, game_id: Uuid) -> bool {
    matches!(self.games.get(&game_id.to_string()), Some(p) if p.ge(&OWNER_PERMISSION))
  }
  
  pub fn can_play(&self, game_id: Uuid) -> bool {
    matches!(self.games.get(&game_id.to_string()), Some(p) if p.ge(&PLAY_PERMISSION))
  }

  pub fn can_view(&self, game_id: Uuid) -> bool {
    matches!(self.games.get(&game_id.to_string()), Some(p) if p.ge(&VIEW_PERMISSION))
  }

  pub fn permission_level(&self, game_id: Uuid) -> i64 {
    match self.games.get(&game_id.to_string()) {
      Some(p) => *p,
      None => 0,
    }
  }

  pub fn custom_claims(&self) -> CustomClaims {
    CustomClaims {
      games: self.games.clone(),
    }
  }
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct ProviderUserInfo {
  pub providerId: String,
  pub displayName: Option<String>,
  pub photoUrl: Option<String>,
  pub federatedId: Option<String>,
  pub email: Option<String>,
  pub rawId: String,
  pub screenName: Option<String>,
  pub phoneNumber: Option<String>,
}

#[serde_as]
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct User {
  pub localId: String,
  pub email: String,
  pub displayName: Option<String>,
  pub language: Option<String>,
  pub photoUrl: Option<String>,
  pub timeZone: Option<String>,
  pub dateOfBirth: Option<String>,
  pub emailVerified: bool,
  pub passwordUpdatedAt: i64,
  pub providerUserInfo: Vec<ProviderUserInfo>,
  pub validSince: String,
  #[serde(default)]
  pub disabled: bool,
  #[serde(with = "serde_with::chrono_0_4::datetime_utc_ts_seconds_from_any")]
  pub lastLoginAt: DateTime<Utc>,
  #[serde(with = "serde_with::chrono_0_4::datetime_utc_ts_seconds_from_any")]
  pub createdAt: DateTime<Utc>,
  pub phoneNumber: Option<String>,
  #[serde_as(as = "serde_with::json::JsonString")]
  pub customAttributes: CustomClaims,
  #[serde(default)]
  pub emailLinkSignin: bool,
  pub initialEmail: Option<String>,
  pub lastRefreshAt: String,
}
