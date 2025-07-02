use std::{collections::HashMap, num::ParseIntError};

use reqwest::{IntoUrl, Url};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum Error {
    HttpError(reqwest::Error),
    IdParseError(ParseIntError),
    ResponseParseError(String),
}

#[derive(Debug, Clone)]
/// An authorized client for a deconz server
pub struct DeconzClient {
    /// The url of the deconz server
    url: Url,
    /// The API token for the deconz server
    pub username: String,
    http: reqwest::Client,
}

#[derive(Debug, Clone)]
pub struct Light {
    pub name: String,
    pub id: u32,
}

#[derive(Debug, Clone, Deserialize, Copy)]
pub struct LightState {
    pub on: bool,
    pub reachable: bool,
    pub hue: Option<u16>,
    pub bri: Option<u8>,
    pub sat: Option<u8>,
}

pub trait LightClient {
    async fn get_light_list(&self) -> Result<Vec<Light>, crate::Error>;

    async fn set_on_state(&self, light: &Light, state: bool) -> Result<(), Error>;

    async fn set_light_color(&self, light: &Light, hue: Option<u16>, bri: Option<u8>, sat: Option<u8>)
    -> Result<(), Error>;

    async fn get_light_state(&self, light: &Light) -> Result<LightState, Error>;
}

impl LightClient for DeconzClient {
    async fn get_light_list(&self) -> Result<Vec<Light>, crate::Error> {
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

        let lights: Vec<Light> = lights
            .into_iter()
            .map(|(id, light)| {
                u32::from_str_radix(&id, 10)
                    .map_err(|e| Error::IdParseError(e))
                    .and_then(|id| {
                        Ok(Light {
                            name: light.name,
                            id,
                        })
                    })
            })
            .collect::<Result<Vec<Light>, Error>>()?;

        Ok(lights)
    }

    async fn set_on_state(&self, light: &Light, state: bool) -> Result<(), Error> {
        #[derive(Serialize)]
        struct OnOffReq {
            on: bool,
        }

        let resp = self
            .http
            .put(
                self.url
                    .join(&format!("api/{}/lights/{}/state", self.username, light.id))
                    .unwrap(),
            )
            .json(&OnOffReq { on: state })
            .send()
            .await;
        let _ = resp
            .and_then(|r| r.error_for_status())
            .map_err(|e| Error::HttpError(e))?;

        Ok(())
    }

    async fn set_light_color(
        &self,
        light: &Light,
        hue: Option<u16>,
        bri: Option<u8>,
        sat: Option<u8>,
    ) -> Result<(), Error> {
        #[derive(Serialize)]
        struct ColorChangeReq {
            hue: Option<u16>,
            bri: Option<u8>,
            sat: Option<u8>,
        }

        self.http
            .put(
                self.url
                    .join(&format!("api/{}/lights/{}/state", self.username, light.id))
                    .unwrap(),
            )
            .json(&ColorChangeReq { hue, bri, sat })
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| Error::HttpError(e))?;

        Ok(())
    }
    
    async fn get_light_state(&self, light: &Light) -> Result<LightState, Error> {
        #[derive(Deserialize)]
        struct OuterLightState {
            state: LightState,
        }

        println!("Loading light state for light id {}", light.id);
        let state = self
            .http
            .get(
                self.url
                    .join(&format!("api/{}/lights/{}", self.username, light.id))
                    .unwrap(),
            )
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| Error::HttpError(e))?
            .json::<OuterLightState>()
            .await
            .map_err(|e| Error::ResponseParseError(e.to_string()))?;

        Ok(state.state)
    }
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
}



pub struct DemoLightClient{

}

impl LightClient for DemoLightClient{
    async fn get_light_list(&self) -> Result<Vec<Light>, crate::Error> {
        Ok(vec![
            Light{
                name: String::from("Bathroom light"),
                id: 1
            },
            Light{
                name: String::from("Outside lighting"),
                id: 2,
            },
            Light{
                name: String::from("Studio lamp"),
                id: 3,
            }
        ])
    }

    async fn set_on_state(&self, light: &Light, state: bool) -> Result<(), Error> {
        Ok(())
    }

    async fn set_light_color(&self, light: &Light, hue: Option<u16>, bri: Option<u8>, sat: Option<u8>)
    -> Result<(), Error> {
        Ok(())
    }

    async fn get_light_state(&self, light: &Light) -> Result<LightState, Error> {
        Ok(LightState { on: true, reachable: true, hue: Some(0), bri: Some(255), sat: Some(200) })
    }
}