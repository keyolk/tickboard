use crate::{
    config,
    models::{EconomicIndicator, FxRate},
};

pub async fn fetch_indicators() -> Vec<EconomicIndicator> {
    let client = super::http_client();
    futures::future::join_all(config::INDICATORS.iter().map(|(symbol, name, unit)| {
        let client = client.clone();
        async move {
            let series = super::yahoo::fetch_chart(&client, symbol, "5d", "1d")
                .await
                .ok()?;
            let value = series.latest_close()?;
            let prev = series.previous_close().unwrap_or(value);
            let change = value - prev;
            Some(EconomicIndicator {
                symbol: (*symbol).to_string(),
                name: (*name).to_string(),
                value,
                change,
                change_pct: if prev != 0.0 {
                    change / prev * 100.0
                } else {
                    0.0
                },
                unit: (*unit).to_string(),
                last_updated: Some(chrono::Local::now()),
            })
        }
    }))
    .await
    .into_iter()
    .flatten()
    .collect()
}

pub async fn fetch_fx_rates() -> Vec<FxRate> {
    let client = super::http_client();
    let (
        usd_krw_7d,
        usd_krw_30d,
        usd_krw_90d,
        usd_krw_1y,
        eur_usd_7d,
        eur_usd_30d,
        eur_usd_90d,
        eur_usd_1y,
        usd_jpy_7d,
        usd_jpy_30d,
        usd_jpy_90d,
        usd_jpy_1y,
    ) = tokio::join!(
        super::yahoo::fetch_chart(&client, "KRW=X", "7d", "1d"),
        super::yahoo::fetch_chart(&client, "KRW=X", "1mo", "1d"),
        super::yahoo::fetch_chart(&client, "KRW=X", "3mo", "1d"),
        super::yahoo::fetch_chart(&client, "KRW=X", "1y", "1d"),
        super::yahoo::fetch_chart(&client, "EURUSD=X", "7d", "1d"),
        super::yahoo::fetch_chart(&client, "EURUSD=X", "1mo", "1d"),
        super::yahoo::fetch_chart(&client, "EURUSD=X", "3mo", "1d"),
        super::yahoo::fetch_chart(&client, "EURUSD=X", "1y", "1d"),
        super::yahoo::fetch_chart(&client, "JPY=X", "7d", "1d"),
        super::yahoo::fetch_chart(&client, "JPY=X", "1mo", "1d"),
        super::yahoo::fetch_chart(&client, "JPY=X", "3mo", "1d"),
        super::yahoo::fetch_chart(&client, "JPY=X", "1y", "1d"),
    );

    let (
        Ok(usd_krw_7d),
        Ok(usd_krw_30d),
        Ok(usd_krw_90d),
        Ok(usd_krw_1y),
        Ok(eur_usd_7d),
        Ok(eur_usd_30d),
        Ok(eur_usd_90d),
        Ok(eur_usd_1y),
        Ok(usd_jpy_7d),
        Ok(usd_jpy_30d),
        Ok(usd_jpy_90d),
        Ok(usd_jpy_1y),
    ) = (
        usd_krw_7d,
        usd_krw_30d,
        usd_krw_90d,
        usd_krw_1y,
        eur_usd_7d,
        eur_usd_30d,
        eur_usd_90d,
        eur_usd_1y,
        usd_jpy_7d,
        usd_jpy_30d,
        usd_jpy_90d,
        usd_jpy_1y,
    )
    else {
        return Vec::new();
    };

    let usd_krw = SeriesSet {
        history_7d: clean_series(&usd_krw_7d.closes),
        history_30d: clean_series(&usd_krw_30d.closes),
        history_90d: clean_series(&usd_krw_90d.closes),
        history_1y: clean_series(&usd_krw_1y.closes),
    };
    let eur_usd = SeriesSet {
        history_7d: clean_series(&eur_usd_7d.closes),
        history_30d: clean_series(&eur_usd_30d.closes),
        history_90d: clean_series(&eur_usd_90d.closes),
        history_1y: clean_series(&eur_usd_1y.closes),
    };
    let usd_jpy = SeriesSet {
        history_7d: clean_series(&usd_jpy_7d.closes),
        history_30d: clean_series(&usd_jpy_30d.closes),
        history_90d: clean_series(&usd_jpy_90d.closes),
        history_1y: clean_series(&usd_jpy_1y.closes),
    };

    let eur_krw = SeriesSet::mul(&eur_usd, &usd_krw);
    let jpy_krw = SeriesSet::scaled_div(100.0, &usd_krw, &usd_jpy);
    let krw_usd = usd_krw.reciprocal();
    let usd_eur = eur_usd.reciprocal();
    let krw_eur = eur_krw.reciprocal();
    let jpy_usd = usd_jpy.reciprocal_scaled(100.0);
    let eur_jpy = SeriesSet::mul(&eur_usd, &usd_jpy);
    let jpy_eur = eur_jpy.reciprocal_scaled(100.0);
    let krw_jpy = SeriesSet::div(&usd_jpy, &usd_krw);

    [
        build_fx_rate("USDKRW", "USD/KRW", "USD", "KRW", usd_krw.clone()),
        build_fx_rate("EURKRW", "EUR/KRW", "EUR", "KRW", eur_krw.clone()),
        build_fx_rate("JPYKRW100", "JPY/KRW", "JPY", "KRW", jpy_krw.clone()),
        build_fx_rate("KRWUSD", "KRW/USD", "KRW", "USD", krw_usd.clone()),
        build_fx_rate("EURUSD", "EUR/USD", "EUR", "USD", eur_usd.clone()),
        build_fx_rate("JPYUSD100", "JPY/USD", "JPY", "USD", jpy_usd.clone()),
        build_fx_rate("KRWEUR", "KRW/EUR", "KRW", "EUR", krw_eur.clone()),
        build_fx_rate("USDEUR", "USD/EUR", "USD", "EUR", usd_eur.clone()),
        build_fx_rate("JPYEUR100", "JPY/EUR", "JPY", "EUR", jpy_eur.clone()),
        build_fx_rate("KRWJPY", "KRW/JPY", "KRW", "JPY", krw_jpy.clone()),
        build_fx_rate("USDJPY", "USD/JPY", "USD", "JPY", usd_jpy.clone()),
        build_fx_rate("EURJPY", "EUR/JPY", "EUR", "JPY", eur_jpy.clone()),
    ]
    .into_iter()
    .flatten()
    .collect()
}

#[derive(Debug, Clone)]
struct SeriesSet {
    history_7d: Vec<f64>,
    history_30d: Vec<f64>,
    history_90d: Vec<f64>,
    history_1y: Vec<f64>,
}

impl SeriesSet {
    fn mul(left: &Self, right: &Self) -> Self {
        Self {
            history_7d: cross_mul(&left.history_7d, &right.history_7d),
            history_30d: cross_mul(&left.history_30d, &right.history_30d),
            history_90d: cross_mul(&left.history_90d, &right.history_90d),
            history_1y: cross_mul(&left.history_1y, &right.history_1y),
        }
    }

    fn div(numerator: &Self, denominator: &Self) -> Self {
        Self {
            history_7d: cross_div(&numerator.history_7d, &denominator.history_7d),
            history_30d: cross_div(&numerator.history_30d, &denominator.history_30d),
            history_90d: cross_div(&numerator.history_90d, &denominator.history_90d),
            history_1y: cross_div(&numerator.history_1y, &denominator.history_1y),
        }
    }

    fn scaled_div(scale: f64, numerator: &Self, denominator: &Self) -> Self {
        Self {
            history_7d: cross_div_scaled(scale, &numerator.history_7d, &denominator.history_7d),
            history_30d: cross_div_scaled(scale, &numerator.history_30d, &denominator.history_30d),
            history_90d: cross_div_scaled(scale, &numerator.history_90d, &denominator.history_90d),
            history_1y: cross_div_scaled(scale, &numerator.history_1y, &denominator.history_1y),
        }
    }

    fn reciprocal(&self) -> Self {
        Self {
            history_7d: reciprocal(&self.history_7d),
            history_30d: reciprocal(&self.history_30d),
            history_90d: reciprocal(&self.history_90d),
            history_1y: reciprocal(&self.history_1y),
        }
    }

    fn reciprocal_scaled(&self, scale: f64) -> Self {
        Self {
            history_7d: reciprocal_scaled(&self.history_7d, scale),
            history_30d: reciprocal_scaled(&self.history_30d, scale),
            history_90d: reciprocal_scaled(&self.history_90d, scale),
            history_1y: reciprocal_scaled(&self.history_1y, scale),
        }
    }
}

fn build_fx_rate(
    symbol: &str,
    pair: &str,
    base_currency: &str,
    quote_currency: &str,
    histories: SeriesSet,
) -> Option<FxRate> {
    let value = *histories.history_7d.last()?;
    let prev = if histories.history_7d.len() >= 2 {
        histories.history_7d[histories.history_7d.len() - 2]
    } else {
        value
    };
    let change = value - prev;
    Some(FxRate {
        symbol: symbol.to_string(),
        pair: pair.to_string(),
        quote_currency: quote_currency.to_string(),
        base_currency: base_currency.to_string(),
        value,
        change,
        change_pct: if prev != 0.0 {
            change / prev * 100.0
        } else {
            0.0
        },
        history_7d: histories.history_7d,
        history_30d: histories.history_30d,
        history_90d: histories.history_90d,
        history_1y: histories.history_1y,
        last_updated: Some(chrono::Local::now()),
    })
}

fn clean_series(values: &[f64]) -> Vec<f64> {
    values
        .iter()
        .copied()
        .filter(|value| value.is_finite() && *value > 0.0)
        .collect()
}

fn cross_mul(left: &[f64], right: &[f64]) -> Vec<f64> {
    let len = left.len().min(right.len());
    left[left.len().saturating_sub(len)..]
        .iter()
        .zip(right[right.len().saturating_sub(len)..].iter())
        .filter_map(|(lhs, rhs)| {
            if lhs.is_finite() && rhs.is_finite() && *lhs > 0.0 && *rhs > 0.0 {
                Some(lhs * rhs)
            } else {
                None
            }
        })
        .collect()
}

fn cross_div(numerator: &[f64], denominator: &[f64]) -> Vec<f64> {
    let len = numerator.len().min(denominator.len());
    numerator[numerator.len().saturating_sub(len)..]
        .iter()
        .zip(denominator[denominator.len().saturating_sub(len)..].iter())
        .filter_map(|(num, den)| {
            if num.is_finite() && den.is_finite() && *num > 0.0 && *den > 0.0 {
                Some(num / den)
            } else {
                None
            }
        })
        .collect()
}

fn cross_div_scaled(scale: f64, numerator: &[f64], denominator: &[f64]) -> Vec<f64> {
    let len = numerator.len().min(denominator.len());
    numerator[numerator.len().saturating_sub(len)..]
        .iter()
        .zip(denominator[denominator.len().saturating_sub(len)..].iter())
        .filter_map(|(num, den)| {
            if num.is_finite() && den.is_finite() && *num > 0.0 && *den > 0.0 {
                Some(scale * num / den)
            } else {
                None
            }
        })
        .collect()
}

fn reciprocal(values: &[f64]) -> Vec<f64> {
    values
        .iter()
        .copied()
        .filter_map(|value| {
            if value.is_finite() && value > 0.0 {
                Some(1.0 / value)
            } else {
                None
            }
        })
        .collect()
}

fn reciprocal_scaled(values: &[f64], scale: f64) -> Vec<f64> {
    values
        .iter()
        .copied()
        .filter_map(|value| {
            if value.is_finite() && value > 0.0 {
                Some(scale / value)
            } else {
                None
            }
        })
        .collect()
}
