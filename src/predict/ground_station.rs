pub const EARTH_ROTATION_RAD_S: f64 = 7.292_115e-5;

#[derive(Debug, Clone, Copy)]
pub struct GroundStation {
    pub latitude_deg: f64,
    pub longitude_deg: f64,
    pub altitude_m: f64,
}

impl Default for GroundStation {
    fn default() -> Self {
        Self {
            latitude_deg: 0.0,
            longitude_deg: 0.0,
            altitude_m: 0.0,
        }
    }
}

impl GroundStation {
    pub fn from_coordinates(coordinates: &str, altitude_m: Option<f64>) -> Option<Self> {
        let parts: Vec<_> = coordinates.split(',').map(|s| s.trim()).collect();
        if parts.len() < 2 {
            return None;
        }
        let lat = parts[0].parse().ok()?;
        let lon = parts[1].parse().ok()?;
        let alt = altitude_m.unwrap_or(0.0);
        Some(Self {
            latitude_deg: lat,
            longitude_deg: lon,
            altitude_m: alt,
        })
    }

    pub fn lat_rad(&self) -> f64 {
        self.latitude_deg.to_radians()
    }

    pub fn lon_rad(&self) -> f64 {
        self.longitude_deg.to_radians()
    }

    pub fn position_ecef_km(&self) -> [f64; 3] {
        // WGS-84 constants
        let a = 6378.137;
        let e2 = 0.00669437999014;
        let lat = self.lat_rad();
        let lon = self.lon_rad();
        let sin_lat = lat.sin();
        let cos_lat = lat.cos();
        let sin_lon = lon.sin();
        let cos_lon = lon.cos();
        let n = a / (1.0 - e2 * sin_lat * sin_lat).sqrt();
        let alt_km = self.altitude_m / 1000.0;
        let x = (n + alt_km) * cos_lat * cos_lon;
        let y = (n + alt_km) * cos_lat * sin_lon;
        let z = (n * (1.0 - e2) + alt_km) * sin_lat;
        [x, y, z]
    }

    pub fn velocity_ecef_km_s(&self) -> [f64; 3] {
        let pos = self.position_ecef_km();
        [
            -EARTH_ROTATION_RAD_S * pos[1],
            EARTH_ROTATION_RAD_S * pos[0],
            0.0,
        ]
    }
}
