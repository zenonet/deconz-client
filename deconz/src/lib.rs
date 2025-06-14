use std::{collections::HashMap, num::ParseIntError};

use reqwest::{IntoUrl, Url};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum Error {
    HttpError(reqwest::Error),
    IdParseError(ParseIntError),
}

#[derive(Debug)]
/// An authorized client for a deconz server
pub struct DeconzClient {
    /// The url of the deconz server
    url: Url,
    /// The API token for the deconz server
    username: String,
    http: reqwest::Client,
}

#[derive(Debug)]
pub struct Light {
    pub name: String,
    pub id: u32,
}

impl DeconzClient {
    /// Creates a new `DeconzClient` by logging in with via the link button
    pub async fn login_with_link_button<U: IntoUrl>(url: U) -> Result<DeconzClient, crate::Error> {
        let http = reqwest::ClientBuilder::new()
            .build()
            .map_err(|e| Error::HttpError(e))?;

        #[derive(Serialize)]
        struct LinkButtonLoginRequest {
            devicetype: String,
        }

        let url = url.into_url().map_err(|e| crate::Error::HttpError(e))?;

        let resp = http
            .post(url.join("api").unwrap())
            .json(&LinkButtonLoginRequest {
                devicetype: String::from("deconz-rs"),
            })
            .send()
            .await
            .and_then(|d| d.error_for_status())
            .map_err(|e| crate::Error::HttpError(e))?;

        #[derive(Deserialize)]
        struct Success {
            username: String,
        }

        #[derive(Deserialize)]
        struct LinkButtonLoginResponse {
            success: Success,
        }

        //println!("{}", resp.text().await.unwrap());

        let resp = resp
            .json::<[LinkButtonLoginResponse; 1]>()
            .await
            .map_err(|e| crate::Error::HttpError(e))?;

        let username = resp.into_iter().next().unwrap().success.username;

        let c = DeconzClient {
            http,
            url,
            username,
        };

        Ok(c)
    }

    /// Creates a new `DeconzClient` from an existing token aka. username
    /// <div class="warning">This method does not validate the token</div>
    pub fn login_with_token<U: IntoUrl>(
        url: U,
        token: String,
    ) -> Result<DeconzClient, crate::Error> {
        let http = reqwest::ClientBuilder::new()
            .build()
            .map_err(|e| Error::HttpError(e))?;

        let url = url.into_url().map_err(|e| Error::HttpError(e))?;

        let c = DeconzClient {
            http,
            url,
            username: token,
        };

        Ok(c)
    }

    pub async fn get_light_list(&self) -> Result<Vec<Light>, crate::Error> {
        let resp = self
            .http
            .get(
                self.url
                    .join(&format!("api/{}/lights", self.username))
                    .unwrap(),
            )
            .send()
            .await;

        let resp = resp
            .and_then(|r| r.error_for_status())
            .map_err(|e| Error::HttpError(e))?;

        #[derive(Deserialize)]
        struct LightWithoutId {
            name: String,
        }

        let lights = resp
            .json::<HashMap<String, LightWithoutId>>()
            .await
            .map_err(|e| Error::HttpError(e))?;


        let lights: Vec<Light> = lights.into_iter().map(|(id, light)| {
            u32::from_str_radix(&id, 10)
                .map_err(|e| Error::IdParseError(e))
                .and_then(|id| {
                    Ok(Light {
                        name: light.name,
                        id,
                    })
                })
        }).collect::<Result<Vec<Light>, Error>>()?;

        Ok(lights)
    }
}
