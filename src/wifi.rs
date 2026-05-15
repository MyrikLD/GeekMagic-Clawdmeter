use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    nvs::EspDefaultNvsPartition,
    wifi::{BlockingWifi, ClientConfiguration, Configuration, EspWifi},
};

pub fn connect<'d>(
    modem: impl esp_idf_hal::modem::WifiModemPeripheral + 'd,
    sysloop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
    ssid: &str,
    pass: &str,
) -> anyhow::Result<BlockingWifi<EspWifi<'d>>> {
    let esp_wifi = EspWifi::new(modem, sysloop.clone(), Some(nvs))?;
    let mut wifi = BlockingWifi::wrap(esp_wifi, sysloop)?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: ssid
            .try_into()
            .map_err(|_| anyhow::anyhow!("SSID too long"))?,
        password: pass
            .try_into()
            .map_err(|_| anyhow::anyhow!("password too long"))?,
        ..Default::default()
    }))?;

    wifi.start()?;
    wifi.connect()?;
    wifi.wait_netif_up()?;

    let ip = wifi.wifi().sta_netif().get_ip_info()?;
    log::info!("IP: {}", ip.ip);

    Ok(wifi)
}
