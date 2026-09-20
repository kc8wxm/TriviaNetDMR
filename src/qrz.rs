use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub struct QrzData {
    pub callsign: String,
    pub name: Option<String>,
    pub location: Option<String>,
    pub is_mock: bool,
}

pub struct QrzClient {
    username: Option<String>,
    password: Option<String>,
    session_key: Arc<Mutex<Option<(String, std::time::Instant)>>>,
    client: reqwest::Client,
}

impl QrzClient {
    pub fn new() -> Self {
        let username = std::env::var("QRZ_USERNAME").ok().filter(|s| !s.trim().is_empty());
        let password = std::env::var("QRZ_PASSWORD").ok().filter(|s| !s.trim().is_empty());

        Self {
            username,
            password,
            session_key: Arc::new(Mutex::new(None)),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
        }
    }

    pub fn is_mocked(&self) -> bool {
        self.username.is_none() || self.password.is_none()
    }

    /// Fetches details for a given callsign.
    /// Uses real QRZ XML API if credentials are provided, otherwise falls back to a realistic mock.
    pub async fn lookup(&self, callsign: &str) -> Result<QrzData, String> {
        let call_upper = callsign.trim().to_uppercase();
        if self.is_mocked() {
            return Ok(Self::get_mock_data(&call_upper));
        }

        // Get or refresh session key
        let s_key = match self.get_session_key().await {
            Ok(k) => k,
            Err(_e) => {
                // Fallback to mock on auth error so the app continues working gracefully
                let mut mock = Self::get_mock_data(&call_upper);
                mock.is_mock = true; // Still marked as mock
                return Ok(mock);
            }
        };

        // Query QRZ XML API
        let url = format!(
            "https://xmldata.qrz.com/xml/current/?s={};callsign={}",
            s_key, call_upper
        );

        let resp = self.client.get(&url)
            .header("User-Agent", "TriviaNetDMR/1.0")
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let body = resp.text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        Self::parse_qrz_xml(&call_upper, &body)
    }

    async fn get_session_key(&self) -> Result<String, String> {
        let mut key_guard = self.session_key.lock().await;
        
        // Session keys are typically valid for 24h. We cache for 12 hours to be safe.
        if let Some((ref key, ref instant)) = *key_guard {
            if instant.elapsed() < std::time::Duration::from_secs(12 * 3600) {
                return Ok(key.clone());
            }
        }

        let username = self.username.as_ref().ok_or("No QRZ username")?;
        let password = self.password.as_ref().ok_or("No QRZ password")?;

        let url = format!(
            "https://xmldata.qrz.com/xml/current/?username={};password={};agent=TriviaNetDMR1.0",
            username, password
        );

        let resp = self.client.get(&url)
            .header("User-Agent", "TriviaNetDMR/1.0")
            .send()
            .await
            .map_err(|e| format!("Login request failed: {}", e))?;

        let body = resp.text()
            .await
            .map_err(|e| format!("Login failed to read: {}", e))?;

        // Parse Session Key from Login XML
        let doc = roxmltree::Document::parse(&body)
            .map_err(|e| format!("Login XML parse error: {}", e))?;

        if let Some(err_node) = doc.descendants().find(|n| n.has_tag_name("Error")) {
            return Err(err_node.text().unwrap_or("Unknown login error").to_string());
        }

        let key_node = doc.descendants()
            .find(|n| n.has_tag_name("Key"))
            .ok_or("No Session Key found in login XML")?;

        let key = key_node.text().ok_or("Empty Session Key")?.to_string();
        *key_guard = Some((key.clone(), std::time::Instant::now()));

        Ok(key)
    }

    fn parse_qrz_xml(callsign: &str, xml: &str) -> Result<QrzData, String> {
        let doc = roxmltree::Document::parse(xml)
            .map_err(|e| format!("XML parse error: {}", e))?;

        // Check for session/API level errors (like expired session key)
        if let Some(err_node) = doc.descendants().find(|n| n.has_tag_name("Error")) {
            let err_txt = err_node.text().unwrap_or("Unknown error");
            if err_txt.contains("Session Timeout") || err_txt.contains("not found") {
                return Err(err_txt.to_string());
            }
        }

        let call_node = doc.descendants()
            .find(|n| n.has_tag_name("Callsign"))
            .ok_or_else(|| format!("Callsign {} not found in QRZ database", callsign))?;

        let fname = call_node.descendants().find(|n| n.has_tag_name("fname")).and_then(|n| n.text().map(String::from));
        let name = call_node.descendants().find(|n| n.has_tag_name("name")).and_then(|n| n.text().map(String::from));
        let addr2 = call_node.descendants().find(|n| n.has_tag_name("addr2")).and_then(|n| n.text().map(String::from));
        let state = call_node.descendants().find(|n| n.has_tag_name("state")).and_then(|n| n.text().map(String::from));
        let country = call_node.descendants().find(|n| n.has_tag_name("country")).and_then(|n| n.text().map(String::from));

        let full_name = match (fname, name) {
            (Some(f), Some(n)) => Some(format!("{} {}", f, n)),
            (Some(f), None) => Some(f),
            (None, Some(n)) => Some(n),
            _ => None,
        };

        let location = match (addr2, state, country) {
            (Some(city), Some(st), Some(co)) => Some(format!("{}, {}, {}", city, st, co)),
            (Some(city), Some(st), None) => Some(format!("{}, {}", city, st)),
            (Some(city), None, Some(co)) => Some(format!("{}, {}", city, co)),
            (None, Some(st), Some(co)) => Some(format!("{}, {}", st, co)),
            (None, None, Some(co)) => Some(co),
            _ => None,
        };

        Ok(QrzData {
            callsign: callsign.to_string(),
            name: full_name,
            location,
            is_mock: false,
        })
    }

    /// Generates beautiful, region-appropriate ham radio mock data
    pub fn get_mock_data(callsign: &str) -> QrzData {
        let call = callsign.trim().to_uppercase();
        
        // Check international prefixes first
        let (name, location) = if call.starts_with("VE") || call.starts_with("VA") {
            ("Maple Leaf", "Toronto, ON, Canada")
        } else if call.starts_with("G") || call.starts_with("M") {
            ("Arthur Pendragon", "London, UK")
        } else if call.starts_with("DL") {
            ("Dieter Munich", "Berlin, Germany")
        } else if call.starts_with("JA") {
            ("Yoshi Tokyo", "Tokyo, Japan")
        } else if call.starts_with("F") {
            ("Pierre Paris", "Paris, France")
        } else {
            // US and other region fallback based on digit
            let region_num = call.chars().find(|c| c.is_ascii_digit()).and_then(|c| c.to_digit(10));
            match region_num {
                Some(1) => ("Alice Newhaven", "Boston, MA, USA"),
                Some(2) => ("Bob Jersey", "Newark, NJ, USA"),
                Some(3) => ("Charlie Keystone", "Philadelphia, PA, USA"),
                Some(4) => ("David Dixie", "Atlanta, GA, USA"),
                Some(5) => ("Emma Ranger", "Austin, TX, USA"),
                Some(6) => ("Frank Golden", "San Francisco, CA, USA"),
                Some(7) => ("George Cascade", "Seattle, WA, USA"),
                Some(8) => ("Hannah Buckeye", "Cleveland, OH, USA"),
                Some(9) => ("Ian Badger", "Madison, WI, USA"),
                Some(0) => ("Jack Rockies", "Denver, CO, USA"),
                _ => ("Global Op", "Unknown DXCC"),
            }
        };

        QrzData {
            callsign: call,
            name: Some(name.to_string()),
            location: Some(location.to_string()),
            is_mock: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_data_generation() {
        let data1 = QrzClient::get_mock_data("K1ABC");
        assert_eq!(data1.name.unwrap(), "Alice Newhaven");
        assert_eq!(data1.location.unwrap(), "Boston, MA, USA");
        assert!(data1.is_mock);

        let data6 = QrzClient::get_mock_data("W6XYZ");
        assert_eq!(data6.name.unwrap(), "Frank Golden");
        assert_eq!(data6.location.unwrap(), "San Francisco, CA, USA");

        let data_ve = QrzClient::get_mock_data("VE3AAA");
        assert_eq!(data_ve.name.unwrap(), "Maple Leaf");
        assert_eq!(data_ve.location.unwrap(), "Toronto, ON, Canada");
    }

    #[test]
    fn test_parse_xml() {
        let xml = r#"
        <QRZDatabase version="1.34">
          <Callsign>
            <call>W1AW</call>
            <fname>Hiram</fname>
            <name>Maxim</name>
            <addr2>Newington</addr2>
            <state>CT</state>
            <country>United States</country>
          </Callsign>
        </QRZDatabase>
        "#;

        let res = QrzClient::parse_qrz_xml("W1AW", xml).unwrap();
        assert_eq!(res.name.unwrap(), "Hiram Maxim");
        assert_eq!(res.location.unwrap(), "Newington, CT, United States");
        assert!(!res.is_mock);
    }
}
