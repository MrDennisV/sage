use chia_protocol::{Bytes32, Coin};
use chia_sdk_driver::{OptionType, OptionUnderlying};

use crate::{
    Asset, AssetKind, CoinKind, CoinRow, Convert, Database, DatabaseTx, Result, SqlAccess,
    SqlExecutor, SqlRow, SqlValue, is_valid_asset_id, puzzle_hash_from_address, sql_file,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OptionSortMode {
    Name,
    CreatedHeight,
    ExpirationSeconds,
}

#[derive(Debug, Clone, Copy)]
pub struct OptionCoinInfo {
    pub underlying_coin_hash: Bytes32,
    pub underlying_delegated_puzzle_hash: Bytes32,
    pub strike_asset_hash: Bytes32,
    pub strike_amount: u64,
}

#[derive(Debug, Clone)]
pub struct OptionRow {
    pub asset: Asset,
    pub underlying_asset: Asset,
    pub underlying_amount: u64,
    pub strike_asset: Asset,
    pub strike_amount: u64,
    pub expiration_seconds: u64,
    pub coin_row: CoinRow,
    pub underlying_coin_id: Bytes32,
}

#[derive(Debug, Clone, Copy)]
pub struct OptionOfferInfo {
    pub underlying_coin_hash: Bytes32,
    pub underlying_delegated_puzzle_hash: Bytes32,
}

#[derive(Debug, Clone)]
pub struct OptionAssetsRow {
    pub underlying_asset: Asset,
    pub underlying_amount: u64,
    pub strike_asset: Asset,
    pub strike_amount: u64,
    pub expiration_seconds: u64,
}

impl<E: SqlExecutor> Database<E> {
    pub async fn owned_options(
        &self,
        limit: u32,
        offset: u32,
        sort_mode: OptionSortMode,
        ascending: bool,
        find_value: Option<String>,
        include_hidden: bool,
    ) -> Result<(Vec<OptionRow>, u32)> {
        owned_options(
            &self.executor,
            limit,
            offset,
            sort_mode,
            ascending,
            find_value,
            include_hidden,
        )
        .await
    }

    pub async fn wallet_option(&self, launcher_id: Bytes32) -> Result<Option<OptionRow>> {
        wallet_option(&self.executor, launcher_id).await
    }

    pub async fn option_assets(&self, launcher_id: Bytes32) -> Result<Option<OptionAssetsRow>> {
        option_assets(&self.executor, launcher_id).await
    }

    pub async fn option_underlying(
        &self,
        launcher_id: Bytes32,
    ) -> Result<Option<OptionUnderlying>> {
        option_underlying(&self.executor, launcher_id).await
    }

    pub async fn offer_option_info(&self, hash: Bytes32) -> Result<Option<OptionOfferInfo>> {
        offer_option_info(&self.executor, hash).await
    }
}

impl<E: SqlExecutor> DatabaseTx<'_, E> {
    pub async fn insert_option(&mut self, hash: Bytes32, coin_info: &OptionCoinInfo) -> Result<()> {
        insert_option(&mut self.tx, hash, coin_info).await
    }
}

/// Decodes the strike asset columns shared by the option queries.
fn strike_asset_from_row(row: &SqlRow) -> Result<Asset> {
    Ok(Asset {
        hash: row.converted("strike_asset_hash")?,
        name: row.opt_text("strike_asset_name")?,
        ticker: row.opt_text("strike_asset_ticker")?,
        precision: row.i64("strike_asset_precision")?.convert()?,
        icon_url: row.opt_text("strike_asset_icon_url")?,
        description: row.opt_text("strike_asset_description")?,
        is_visible: row.i64("strike_asset_is_visible")? != 0,
        is_sensitive_content: row.i64("strike_asset_is_sensitive_content")? != 0,
        hidden_puzzle_hash: row.opt_converted("strike_asset_hidden_puzzle_hash")?,
        kind: row.i64("strike_asset_kind")?.convert()?,
    })
}

/// Decodes the underlying asset columns shared by the option queries.
fn underlying_asset_from_row(row: &SqlRow) -> Result<Asset> {
    Ok(Asset {
        hash: row.converted("underlying_asset_hash")?,
        name: row.opt_text("underlying_asset_name")?,
        ticker: row.opt_text("underlying_asset_ticker")?,
        precision: row.i64("underlying_asset_precision")?.convert()?,
        icon_url: row.opt_text("underlying_asset_icon_url")?,
        description: row.opt_text("underlying_asset_description")?,
        is_visible: row.i64("underlying_asset_is_visible")? != 0,
        is_sensitive_content: row.i64("underlying_asset_is_sensitive_content")? != 0,
        hidden_puzzle_hash: row.opt_converted("underlying_asset_hidden_puzzle_hash")?,
        kind: row.i64("underlying_asset_kind")?.convert()?,
    })
}

async fn wallet_option(
    mut conn: impl SqlAccess,
    launcher_id: Bytes32,
) -> Result<Option<OptionRow>> {
    conn.fetch_all(
        sql_file!("options/wallet_option.sql"),
        vec![launcher_id.into()],
    )
    .await?
    .first()
    .map(|row| {
        // The `?` suffixes are part of the column aliases in the query, where
        // they mark the columns sqlx should treat as nullable.
        Ok(OptionRow {
            asset: Asset {
                hash: row.converted("asset_hash")?,
                name: row.opt_text("asset_name")?,
                ticker: row.opt_text("asset_ticker")?,
                precision: row.i64("asset_precision")?.convert()?,
                icon_url: row.opt_text("asset_icon_url")?,
                description: row.opt_text("asset_description")?,
                is_visible: row.i64("asset_is_visible")? != 0,
                is_sensitive_content: row.i64("asset_is_sensitive_content")? != 0,
                hidden_puzzle_hash: row.opt_converted("asset_hidden_puzzle_hash")?,
                kind: AssetKind::Option,
            },
            underlying_asset: underlying_asset_from_row(row)?,
            underlying_amount: row.converted("underlying_amount")?,
            strike_asset: strike_asset_from_row(row)?,
            strike_amount: row.converted("strike_amount")?,
            underlying_coin_id: row.converted("underlying_coin_id")?,
            expiration_seconds: row.i64("option_expiration_seconds")?.convert()?,
            coin_row: CoinRow {
                coin: Coin::new(
                    row.converted("parent_coin_hash")?,
                    row.converted("puzzle_hash")?,
                    row.converted("amount")?,
                ),
                p2_puzzle_hash: row.converted("p2_puzzle_hash")?,
                kind: CoinKind::Option,
                mempool_item_hash: None,
                offer_hash: row.opt_converted("offer_hash?")?,
                clawback_timestamp: row.opt_i64("clawback_timestamp?")?.convert()?,
                created_height: row.opt_i64("created_height")?.convert()?,
                spent_height: row.opt_i64("spent_height")?.convert()?,
                created_timestamp: row.opt_i64("created_timestamp")?.convert()?,
                spent_timestamp: row.opt_i64("spent_timestamp")?.convert()?,
            },
        })
    })
    .transpose()
}

async fn option_assets(
    mut conn: impl SqlAccess,
    launcher_id: Bytes32,
) -> Result<Option<OptionAssetsRow>> {
    conn.fetch_all(
        sql_file!("options/option_assets.sql"),
        vec![launcher_id.into()],
    )
    .await?
    .first()
    .map(|row| {
        Ok(OptionAssetsRow {
            underlying_asset: underlying_asset_from_row(row)?,
            underlying_amount: row.converted("underlying_amount")?,
            strike_asset: strike_asset_from_row(row)?,
            strike_amount: row.converted("strike_amount")?,
            expiration_seconds: row.i64("expiration_seconds")?.convert()?,
        })
    })
    .transpose()
}

async fn insert_option(
    mut conn: impl SqlAccess,
    hash: Bytes32,
    coin_info: &OptionCoinInfo,
) -> Result<()> {
    conn.execute(
        sql_file!("options/insert_option.sql"),
        vec![
            hash.into(),
            coin_info.underlying_coin_hash.into(),
            coin_info.underlying_delegated_puzzle_hash.into(),
            coin_info.strike_asset_hash.into(),
            coin_info.strike_amount.to_be_bytes().to_vec().into(),
        ],
    )
    .await?;

    Ok(())
}

async fn option_underlying(
    mut conn: impl SqlAccess,
    launcher_id: Bytes32,
) -> Result<Option<OptionUnderlying>> {
    let rows = conn
        .fetch_all(
            sql_file!("options/option_underlying.sql"),
            vec![launcher_id.into()],
        )
        .await?;

    let Some(row) = rows.first() else {
        return Ok(None);
    };

    let asset_hash: Bytes32 = row.converted("strike_asset_hash")?;
    let amount: u64 = row.converted("strike_amount")?;
    let hidden_puzzle_hash: Option<Bytes32> = row.opt_converted("strike_hidden_puzzle_hash")?;

    Ok(Some(OptionUnderlying::new(
        launcher_id,
        row.converted("creator_puzzle_hash")?,
        row.i64("expiration_seconds")?.convert()?,
        row.converted("underlying_amount")?,
        if asset_hash == Bytes32::default() {
            OptionType::Xch { amount }
        } else if let Some(hidden_puzzle_hash) = hidden_puzzle_hash {
            OptionType::RevocableCat {
                asset_id: asset_hash,
                hidden_puzzle_hash,
                amount,
            }
        } else {
            OptionType::Cat {
                asset_id: asset_hash,
                amount,
            }
        },
    )))
}

async fn offer_option_info(
    mut conn: impl SqlAccess,
    hash: Bytes32,
) -> Result<Option<OptionOfferInfo>> {
    conn.fetch_all(sql_file!("options/offer_option_info.sql"), vec![hash.into()])
        .await?
        .first()
        .map(|row| {
            Ok(OptionOfferInfo {
                underlying_coin_hash: row.converted("underlying_coin_hash")?,
                underlying_delegated_puzzle_hash: row
                    .converted("underlying_delegated_puzzle_hash")?,
            })
        })
        .transpose()
}

async fn owned_options(
    mut conn: impl SqlAccess,
    limit: u32,
    offset: u32,
    sort_mode: OptionSortMode,
    ascending: bool,
    find_value: Option<String>,
    include_hidden: bool,
) -> Result<(Vec<OptionRow>, u32)> {
    let mut sql = "SELECT
            asset_hash, asset_name, asset_ticker, asset_precision, asset_icon_url,
            asset_description, asset_is_visible, asset_is_sensitive_content,
            asset_hidden_puzzle_hash, owned_coins.created_height, owned_coins.spent_height,
            owned_coins.parent_coin_hash, owned_coins.puzzle_hash, owned_coins.amount, owned_coins.p2_puzzle_hash,
            offer_hash, created_timestamp, spent_timestamp,
            clawback_expiration_seconds AS clawback_timestamp,
            p2_options.expiration_seconds AS option_expiration_seconds,

            strike_asset.hash AS strike_asset_hash, strike_asset.name AS strike_asset_name,
            strike_asset.ticker AS strike_asset_ticker, strike_asset.precision AS strike_asset_precision,
            strike_asset.icon_url AS strike_asset_icon_url, strike_asset.description AS strike_asset_description,
            strike_asset.is_visible AS strike_asset_is_visible, strike_asset.is_sensitive_content AS strike_asset_is_sensitive_content,
            strike_asset.hidden_puzzle_hash AS strike_asset_hidden_puzzle_hash, strike_asset.kind AS strike_asset_kind,

            underlying_asset.hash AS underlying_asset_hash, underlying_asset.name AS underlying_asset_name,
            underlying_asset.ticker AS underlying_asset_ticker, underlying_asset.precision AS underlying_asset_precision,
            underlying_asset.icon_url AS underlying_asset_icon_url, underlying_asset.description AS underlying_asset_description,
            underlying_asset.is_visible AS underlying_asset_is_visible, underlying_asset.is_sensitive_content AS underlying_asset_is_sensitive_content,
            underlying_asset.hidden_puzzle_hash AS underlying_asset_hidden_puzzle_hash, underlying_asset.kind AS underlying_asset_kind,

            strike_amount, underlying_coin.amount AS underlying_amount, underlying_coin.hash AS underlying_coin_id,

            COUNT(*) OVER() as total_count
        FROM owned_coins
        INNER JOIN options ON options.asset_id = owned_coins.asset_id
        INNER JOIN p2_options ON p2_options.option_asset_id = options.asset_id
        INNER JOIN coins AS underlying_coin ON underlying_coin.id = options.underlying_coin_id
        INNER JOIN assets AS strike_asset ON strike_asset.id = options.strike_asset_id
        INNER JOIN assets AS underlying_asset ON underlying_asset.id = underlying_coin.asset_id
        WHERE 1=1"
        .to_string();

    let mut params = Vec::new();

    if !include_hidden {
        sql.push_str(" AND asset_is_visible = 1");
    }

    if let Some(find_value) = find_value {
        sql.push_str(" AND (asset_name LIKE ?");
        params.push(SqlValue::from(format!("%{find_value}%")));
        sql.push_str(" OR asset_ticker LIKE ?");
        params.push(SqlValue::from(format!("%{find_value}%")));
        sql.push_str(" OR underlying_asset.name LIKE ?");
        params.push(SqlValue::from(format!("%{find_value}%")));
        sql.push_str(" OR underlying_asset.ticker LIKE ?");
        params.push(SqlValue::from(format!("%{find_value}%")));
        sql.push_str(" OR strike_asset.name LIKE ?");
        params.push(SqlValue::from(format!("%{find_value}%")));
        sql.push_str(" OR strike_asset.ticker LIKE ?");
        params.push(SqlValue::from(format!("%{find_value}%")));

        // If find_value looks like a valid asset ID (64 hex chars), search by asset hash
        if is_valid_asset_id(&find_value) {
            sql.push_str(" OR asset_hash = X'");
            sql.push_str(&find_value);
            sql.push_str("' OR underlying_asset.hash = X'");
            sql.push_str(&find_value);
            sql.push_str("' OR strike_asset.hash = X'");
            sql.push_str(&find_value);
            sql.push('\'');
        }

        // If find_value looks like a valid address, search by puzzle hash
        if let Some(puzzle_hash) = puzzle_hash_from_address(&find_value) {
            sql.push_str(" OR asset_hash = X'");
            sql.push_str(&puzzle_hash);
            sql.push_str("' OR underlying_asset.hash = X'");
            sql.push_str(&puzzle_hash);
            sql.push_str("' OR strike_asset.hash = X'");
            sql.push_str(&puzzle_hash);
            sql.push('\'');
        }
        sql.push(')');
    }

    // Add ORDER BY clause based on sort_mode and ascending
    sql.push_str(" ORDER BY ");
    let order_column = match sort_mode {
        OptionSortMode::Name => "asset_name",
        OptionSortMode::CreatedHeight => "owned_coins.created_height",
        OptionSortMode::ExpirationSeconds => "p2_options.expiration_seconds",
    };
    sql.push_str(order_column);

    if ascending {
        sql.push_str(" ASC");
    } else {
        sql.push_str(" DESC");
    }

    sql.push_str(" LIMIT ?");
    params.push(SqlValue::from(limit));
    sql.push_str(" OFFSET ?");
    params.push(SqlValue::from(offset));

    let rows = conn.fetch_all(&sql, params).await?;

    let total_count: u32 = rows
        .first()
        .map_or(Ok(0), |row| row.i64("total_count")?.convert())?;

    let options = rows
        .iter()
        .map(|row| {
            Ok(OptionRow {
                asset: Asset {
                    hash: row.converted("asset_hash")?,
                    name: row.opt_text("asset_name")?,
                    ticker: row.opt_text("asset_ticker")?,
                    precision: row.i64("asset_precision")?.convert()?,
                    icon_url: row.opt_text("asset_icon_url")?,
                    description: row.opt_text("asset_description")?,
                    is_visible: row.i64("asset_is_visible")? != 0,
                    is_sensitive_content: row.i64("asset_is_sensitive_content")? != 0,
                    hidden_puzzle_hash: row.opt_converted("asset_hidden_puzzle_hash")?,
                    kind: AssetKind::Option,
                },
                underlying_amount: row.converted("underlying_amount")?,
                underlying_coin_id: row.converted("underlying_coin_id")?,
                underlying_asset: Asset {
                    hash: row.converted("underlying_asset_hash")?,
                    name: row.opt_text("underlying_asset_name")?,
                    ticker: row.opt_text("underlying_asset_ticker")?,
                    precision: row.i64("underlying_asset_precision")?.convert()?,
                    icon_url: row.opt_text("underlying_asset_icon_url")?,
                    description: row.opt_text("underlying_asset_description")?,
                    is_visible: row.i64("underlying_asset_is_visible")? != 0,
                    is_sensitive_content: row.i64("underlying_asset_is_sensitive_content")? != 0,
                    hidden_puzzle_hash: row.opt_converted("underlying_asset_hidden_puzzle_hash")?,
                    kind: row
                        .opt_i64("underlying_asset_kind")?
                        .map(Convert::convert)
                        .transpose()?
                        .unwrap_or(AssetKind::Token),
                },
                strike_amount: row.converted("strike_amount")?,
                strike_asset: Asset {
                    hash: row.converted("strike_asset_hash")?,
                    name: row.opt_text("strike_asset_name")?,
                    ticker: row.opt_text("strike_asset_ticker")?,
                    precision: row.i64("strike_asset_precision")?.convert()?,
                    icon_url: row.opt_text("strike_asset_icon_url")?,
                    description: row.opt_text("strike_asset_description")?,
                    is_visible: row.i64("strike_asset_is_visible")? != 0,
                    is_sensitive_content: row.i64("strike_asset_is_sensitive_content")? != 0,
                    hidden_puzzle_hash: row.opt_converted("strike_asset_hidden_puzzle_hash")?,
                    kind: row
                        .opt_i64("strike_asset_kind")?
                        .map(Convert::convert)
                        .transpose()?
                        .unwrap_or(AssetKind::Token),
                },
                expiration_seconds: row.i64("option_expiration_seconds")?.convert()?,
                coin_row: CoinRow {
                    coin: Coin::new(
                        row.converted("parent_coin_hash")?,
                        row.converted("puzzle_hash")?,
                        row.converted("amount")?,
                    ),
                    p2_puzzle_hash: row.converted("p2_puzzle_hash")?,
                    kind: CoinKind::Option,
                    mempool_item_hash: None,
                    offer_hash: row.opt_converted("offer_hash")?,
                    clawback_timestamp: row.opt_i64("clawback_timestamp")?.convert()?,
                    created_height: row.opt_i64("created_height")?.convert()?,
                    spent_height: row.opt_i64("spent_height")?.convert()?,
                    created_timestamp: row.opt_i64("created_timestamp")?.convert()?,
                    spent_timestamp: row.opt_i64("spent_timestamp")?.convert()?,
                },
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok((options, total_count))
}
