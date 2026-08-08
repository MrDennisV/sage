use chia_protocol::{Bytes32, Coin};

use crate::{
    Asset, AssetKind, Convert, Database, DatabaseError, Result, SqlAccess, SqlExecutor, SqlRow,
    SqlValue, sql_file,
};

#[derive(Debug, Clone)]
pub struct Transaction {
    pub height: u32,
    pub timestamp: Option<u64>,
    pub spent: Vec<TransactionCoin>,
    pub created: Vec<TransactionCoin>,
}

#[derive(Debug, Clone)]
pub struct TransactionCoin {
    pub coin: Coin,
    pub asset: Asset,
    pub p2_puzzle_hash: Option<Bytes32>,
}

impl<E: SqlExecutor> Database<E> {
    pub async fn transaction(&self, height: u32) -> Result<Option<Transaction>> {
        transaction(&self.executor, height).await
    }

    pub async fn transactions(
        &self,
        find_value: Option<String>,
        sort_ascending: bool,
        limit: u32,
        offset: u32,
    ) -> Result<(Vec<Transaction>, u32)> {
        transactions(&self.executor, find_value, sort_ascending, limit, offset).await
    }
}

// Helper function to create a TransactionCoin from a database row
fn create_transaction_coin(row: &SqlRow) -> Result<TransactionCoin> {
    let coin = Coin::new(
        row.converted("parent_coin_hash")?,
        row.converted("puzzle_hash")?,
        row.converted("amount")?,
    );

    let asset = Asset {
        hash: row.converted("asset_hash")?,
        name: row.opt_text("asset_name")?,
        ticker: row.opt_text("asset_ticker")?,
        precision: row.i64("asset_precision")?.convert()?,
        icon_url: row.opt_text("asset_icon_url")?,
        description: row.opt_text("asset_description")?,
        is_sensitive_content: row.i64("asset_is_sensitive_content")? != 0,
        is_visible: row.i64("asset_is_visible")? != 0,
        kind: row
            .opt_i64("asset_kind")?
            .map(Convert::convert)
            .transpose()?
            .unwrap_or(AssetKind::Token),
        hidden_puzzle_hash: row.opt_converted("asset_hidden_puzzle_hash")?,
    };

    let p2_puzzle_hash = row.opt_converted("p2_puzzle_hash")?;

    Ok(TransactionCoin {
        coin,
        asset,
        p2_puzzle_hash,
    })
}

async fn transaction(mut conn: impl SqlAccess, height: u32) -> Result<Option<Transaction>> {
    let rows = conn
        .fetch_all(
            sql_file!("transactions/transaction.sql"),
            vec![height.into()],
        )
        .await?;

    if rows.is_empty() {
        return Ok(None);
    }

    let mut spent_coins = Vec::new();
    let mut created_coins = Vec::new();
    let mut timestamp = None;

    for row in &rows {
        if timestamp.is_none() {
            timestamp = row.opt_i64("timestamp")?.map(|ts| ts as u64);
        }

        let coin = Coin::new(
            row.converted("parent_coin_hash")?,
            row.converted("puzzle_hash")?,
            row.converted("amount")?,
        );

        let asset = Asset {
            hash: row.converted("asset_hash")?,
            name: row.opt_text("asset_name")?,
            ticker: row.opt_text("asset_ticker")?,
            precision: row.i64("asset_precision")?.convert()?,
            icon_url: row.opt_text("asset_icon_url")?,
            description: row.opt_text("asset_description")?,
            is_sensitive_content: row.i64("asset_is_sensitive_content")? != 0,
            is_visible: row.i64("asset_is_visible")? != 0,
            kind: row.i64("asset_kind")?.convert()?,
            hidden_puzzle_hash: row.opt_converted("asset_hidden_puzzle_hash")?,
        };

        let transaction_coin = TransactionCoin {
            coin,
            asset,
            p2_puzzle_hash: row.opt_converted("p2_puzzle_hash")?,
        };

        // These compare a coin height against the block height, so they are
        // null when the coin has no such height rather than false.
        if row.opt_i64("is_spent_in_block")? == Some(1) {
            spent_coins.push(transaction_coin.clone());
        }

        if row.opt_i64("is_created_in_block")? == Some(1) {
            created_coins.push(transaction_coin);
        }
    }

    Ok(Some(Transaction {
        height,
        timestamp,
        spent: spent_coins,
        created: created_coins,
    }))
}

async fn transactions(
    mut conn: impl SqlAccess,
    find_value: Option<String>,
    sort_ascending: bool,
    limit: u32,
    offset: u32,
) -> Result<(Vec<Transaction>, u32)> {
    let mut sql = "SELECT
            height, timestamp, coin_id, puzzle_hash, parent_coin_hash, amount,
            is_created_in_block, is_spent_in_block, asset_hash, asset_description,
            asset_is_visible, asset_is_sensitive_content, asset_name, asset_icon_url,
            asset_kind, p2_puzzle_hash, asset_ticker, asset_precision, asset_hidden_puzzle_hash,
            COUNT(*) OVER() as total_count
        FROM transaction_coins
        WHERE 1=1"
        .to_string();

    let mut params = Vec::new();

    if let Some(find_value) = find_value {
        sql.push_str(" AND (asset_name LIKE ?");
        params.push(SqlValue::from(format!("%{find_value}%")));
        sql.push_str(" OR asset_ticker LIKE ?");
        params.push(SqlValue::from(format!("%{find_value}%")));

        if is_valid_asset_id(&find_value) {
            sql.push_str(" OR asset_hash = X'");
            sql.push_str(&find_value);
            sql.push('\'');
        }

        // match on nft or did launcher id
        if let Some(puzzle_hash) = puzzle_hash_from_address(&find_value) {
            sql.push_str(" OR asset_hash = X'");
            sql.push_str(&puzzle_hash);
            sql.push('\'');
        }

        // match on height if the find value is parsable as a u32
        if let Ok(height) = find_value.parse::<u32>() {
            sql.push_str(" OR height = ?");
            params.push(SqlValue::from(height));
        }
        sql.push(')');
    }

    if sort_ascending {
        sql.push_str(" ORDER BY height ASC");
    } else {
        sql.push_str(" ORDER BY height DESC");
    }

    sql.push_str(" LIMIT ?");
    params.push(SqlValue::from(limit));
    sql.push_str(" OFFSET ?");
    params.push(SqlValue::from(offset));

    let rows = conn.fetch_all(&sql, params).await?;

    // The count is narrowed to an `i32` first, the way the sqlx query did.
    let total_count: i32 = rows.first().map_or(Ok(0), |row| {
        i32::try_from(row.i64("total_count")?).map_err(DatabaseError::PrecisionLost)
    })?;

    let transactions = group_rows_into_transactions(&rows, sort_ascending)?;

    Ok((transactions, total_count as u32))
}

pub fn is_valid_asset_id(asset_id: &str) -> bool {
    asset_id.len() == 64 && asset_id.chars().all(|c| c.is_ascii_hexdigit())
}

pub fn puzzle_hash_from_address(address: &str) -> Option<String> {
    chia_sdk_utils::Address::decode(address)
        .map(|decoded| hex::encode(decoded.puzzle_hash.as_ref()))
        .ok()
}

// Helper function to group rows by height and create Transaction structs
fn group_rows_into_transactions(rows: &[SqlRow], sort_ascending: bool) -> Result<Vec<Transaction>> {
    use std::collections::HashMap;

    #[allow(clippy::type_complexity)]
    let mut transactions_by_height: HashMap<
        u32,
        (Option<u64>, Vec<TransactionCoin>, Vec<TransactionCoin>),
    > = HashMap::new();

    for row in rows {
        let height: u32 = row.i64("height")?.convert()?;
        let timestamp: Option<i64> = row.opt_i64("timestamp")?;
        let is_spent_in_block = row.opt_i64("is_spent_in_block")?;
        let is_created_in_block = row.opt_i64("is_created_in_block")?;

        let transaction_coin = create_transaction_coin(row)?;

        let entry = transactions_by_height
            .entry(height)
            .or_insert_with(|| (timestamp.map(|ts| ts as u64), Vec::new(), Vec::new()));

        // These compare a coin height against the block height, so they are
        // null when the coin has no such height rather than false.
        if is_spent_in_block == Some(1) {
            entry.1.push(transaction_coin.clone());
        }

        if is_created_in_block == Some(1) {
            entry.2.push(transaction_coin);
        }
    }

    let mut transactions = Vec::new();
    for (height, (timestamp, spent_coins, created_coins)) in transactions_by_height {
        transactions.push(Transaction {
            height,
            timestamp,
            spent: spent_coins,
            created: created_coins,
        });
    }

    // Sort transactions by height to maintain the order from the SQL query
    if sort_ascending {
        transactions.sort_by_key(|tx| tx.height);
    } else {
        transactions.sort_by_key(|tx| std::cmp::Reverse(tx.height));
    }

    Ok(transactions)
}
