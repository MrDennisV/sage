use chia_protocol::{Bytes32, Coin, CoinState};
use chia_puzzle_types::{LineageProof, Proof};
use chia_sdk_driver::{Cat, CatInfo, OptionContract, OptionInfo};

use crate::{
    AssetKind, Convert, Database, DatabaseError, DatabaseTx, Result, SerializedDid,
    SerializedDidInfo, SerializedNft, SerializedNftInfo, SqlAccess, SqlExecutor, SqlRow, SqlValue,
    sql_file,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CoinKind {
    Xch,
    Cat,
    Did,
    Nft,
    Option,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CoinSortMode {
    CoinId,
    Amount,
    CreatedHeight,
    SpentHeight,
    ClawbackTimestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CoinFilterMode {
    All,
    Selectable,
    Owned,
    Spent,
    Clawback,
}

#[derive(Debug, Clone, Copy)]
pub enum AssetFilter {
    Id(Bytes32),
    Nfts,
    Dids,
}

#[derive(Debug, Clone, Copy)]
pub struct CoinRow {
    pub coin: Coin,
    pub p2_puzzle_hash: Bytes32,
    pub kind: CoinKind,
    pub mempool_item_hash: Option<Bytes32>,
    pub offer_hash: Option<Bytes32>,
    pub clawback_timestamp: Option<u64>,
    pub created_height: Option<u32>,
    pub spent_height: Option<u32>,
    pub created_timestamp: Option<u64>,
    pub spent_timestamp: Option<u64>,
}

#[derive(Debug, Clone, Copy)]
pub struct UnsyncedCoin {
    pub coin_state: CoinState,
    pub is_asset_unsynced: bool,
    pub is_children_unsynced: bool,
}

impl<E: SqlExecutor> Database<E> {
    pub async fn coins_by_ids(&self, coin_ids: &[String]) -> Result<Vec<CoinRow>> {
        coins_by_ids(&self.executor, coin_ids).await
    }

    pub async fn coin_records(
        &self,
        asset_filter: AssetFilter,
        limit: u32,
        offset: u32,
        sort_mode: CoinSortMode,
        ascending: bool,
        filter_mode: CoinFilterMode,
    ) -> Result<(Vec<CoinRow>, u32)> {
        coin_records(
            &self.executor,
            asset_filter,
            limit,
            offset,
            sort_mode,
            ascending,
            filter_mode,
        )
        .await
    }

    pub async fn are_coins_spendable(&self, coin_ids: &[String]) -> Result<bool> {
        are_coins_spendable(&self.executor, coin_ids).await
    }

    pub async fn total_coin_count(&self) -> Result<u32> {
        total_coin_count(&self.executor).await
    }

    pub async fn selectable_xch_coin_count(&self) -> Result<u32> {
        selectable_coin_count(&self.executor, Bytes32::default()).await
    }

    pub async fn selectable_cat_coin_count(&self, asset_id: Bytes32) -> Result<u32> {
        selectable_coin_count(&self.executor, asset_id).await
    }

    pub async fn synced_coin_count(&self) -> Result<u32> {
        synced_coin_count(&self.executor).await
    }

    pub async fn xch_balance(&self) -> Result<u128> {
        token_balance(&self.executor, Bytes32::default()).await
    }

    pub async fn cat_balance(&self, asset_id: Bytes32) -> Result<u128> {
        token_balance(&self.executor, asset_id).await
    }

    pub async fn selectable_xch_balance(&self) -> Result<u128> {
        selectable_token_balance(&self.executor, Bytes32::default()).await
    }

    pub async fn selectable_cat_balance(&self, asset_id: Bytes32) -> Result<u128> {
        selectable_token_balance(&self.executor, asset_id).await
    }

    pub async fn spendable_did(&self, launcher_id: Bytes32) -> Result<Option<SerializedDid>> {
        spendable_did(&self.executor, launcher_id).await
    }

    pub async fn spendable_nft(&self, launcher_id: Bytes32) -> Result<Option<SerializedNft>> {
        spendable_nft(&self.executor, launcher_id).await
    }

    pub async fn spendable_option(&self, launcher_id: Bytes32) -> Result<Option<OptionContract>> {
        spendable_option(&self.executor, launcher_id).await
    }

    pub async fn unsynced_coins(&self, limit: usize) -> Result<Vec<UnsyncedCoin>> {
        unsynced_coins(&self.executor, limit).await
    }

    pub async fn update_coin(
        &self,
        coin_id: Bytes32,
        asset_hash: Bytes32,
        p2_puzzle_hash: Bytes32,
    ) -> Result<()> {
        update_coin(&self.executor, coin_id, asset_hash, p2_puzzle_hash).await
    }

    pub async fn subscription_coin_ids(&self) -> Result<Vec<Bytes32>> {
        subscription_coin_ids(&self.executor).await
    }

    pub async fn selectable_xch_coins(&self) -> Result<Vec<Coin>> {
        selectable_xch_coins(&self.executor).await
    }

    pub async fn selectable_cat_coins(&self, asset_id: Bytes32) -> Result<Vec<Cat>> {
        selectable_cat_coins(&self.executor, asset_id).await
    }

    pub async fn coin_kind(&self, coin_id: Bytes32) -> Result<Option<CoinKind>> {
        coin_kind(&self.executor, coin_id).await
    }

    pub async fn xch_coin(&self, coin_id: Bytes32) -> Result<Option<Coin>> {
        xch_coin(&self.executor, coin_id).await
    }

    pub async fn cat_coin(&self, coin_id: Bytes32) -> Result<Option<Cat>> {
        cat_coin(&self.executor, coin_id).await
    }

    pub async fn did_coin(&self, coin_id: Bytes32) -> Result<Option<SerializedDid>> {
        did_coin(&self.executor, coin_id).await
    }

    pub async fn nft_coin(&self, coin_id: Bytes32) -> Result<Option<SerializedNft>> {
        nft_coin(&self.executor, coin_id).await
    }

    pub async fn option_coin(&self, coin_id: Bytes32) -> Result<Option<OptionContract>> {
        option_coin(&self.executor, coin_id).await
    }

    pub async fn did(&self, launcher_id: Bytes32) -> Result<Option<SerializedDid>> {
        did(&self.executor, launcher_id).await
    }

    pub async fn nft(&self, launcher_id: Bytes32) -> Result<Option<SerializedNft>> {
        nft(&self.executor, launcher_id).await
    }

    pub async fn option(&self, launcher_id: Bytes32) -> Result<Option<OptionContract>> {
        option(&self.executor, launcher_id).await
    }

    pub async fn underlying_coin_kind(&self, launcher_id: Bytes32) -> Result<Option<CoinKind>> {
        underlying_coin_kind(&self.executor, launcher_id).await
    }
}

impl<E: SqlExecutor> DatabaseTx<'_, E> {
    pub async fn set_transaction_children_unsynced(
        &mut self,
        mempool_item_id: Bytes32,
    ) -> Result<()> {
        set_transaction_children_unsynced(&mut self.tx, mempool_item_id).await
    }

    pub async fn insert_coin(&mut self, coin_state: CoinState) -> Result<()> {
        insert_coin(&mut self.tx, coin_state).await
    }

    pub async fn is_known_coin(&mut self, coin_id: Bytes32) -> Result<bool> {
        is_known_coin(&mut self.tx, coin_id).await
    }

    pub async fn update_coin(
        &mut self,
        coin_id: Bytes32,
        asset_hash: Bytes32,
        p2_puzzle_hash: Bytes32,
    ) -> Result<()> {
        update_coin(&mut self.tx, coin_id, asset_hash, p2_puzzle_hash).await
    }

    pub async fn set_children_synced(&mut self, coin_id: Bytes32) -> Result<()> {
        set_children_synced(&mut self.tx, coin_id).await
    }

    pub async fn delete_coin(&mut self, coin_id: Bytes32) -> Result<()> {
        delete_coin(&mut self.tx, coin_id).await
    }

    pub async fn insert_lineage_proof(
        &mut self,
        coin_id: Bytes32,
        lineage_proof: LineageProof,
    ) -> Result<()> {
        insert_lineage_proof(&mut self.tx, coin_id, lineage_proof).await
    }
}

/// Joins the hex coin ids into `X'..'` literals, matching how the sqlx
/// `QueryBuilder` inlined them.
fn coin_hash_literals(coin_ids: &[String]) -> String {
    coin_ids
        .iter()
        .map(|coin_id| format!("X'{coin_id}'"))
        .collect::<Vec<_>>()
        .join(", ")
}

async fn are_coins_spendable(mut conn: impl SqlAccess, coin_ids: &[String]) -> Result<bool> {
    if coin_ids.is_empty() {
        return Ok(false);
    }

    let mut sql = "
        SELECT COUNT(*) AS count
        FROM spendable_coins
        WHERE 1=1
        AND coin_hash IN ("
        .to_string();

    sql.push_str(&coin_hash_literals(coin_ids));
    sql.push(')');

    let count = conn
        .fetch_all(&sql, vec![])
        .await?
        .first()
        .ok_or(DatabaseError::RowNotFound)?
        .i64("count")?;

    #[allow(clippy::cast_possible_wrap)]
    Ok(count == coin_ids.len() as i64)
}

async fn insert_coin(mut conn: impl SqlAccess, coin_state: CoinState) -> Result<()> {
    conn.execute(
        sql_file!("coins/insert_coin.sql"),
        vec![
            coin_state.coin.coin_id().into(),
            coin_state.coin.parent_coin_info.into(),
            coin_state.coin.puzzle_hash.into(),
            coin_state.coin.amount.to_be_bytes().to_vec().into(),
            coin_state.created_height.into(),
            coin_state.spent_height.into(),
        ],
    )
    .await?;

    Ok(())
}

async fn is_known_coin(mut conn: impl SqlAccess, coin_id: Bytes32) -> Result<bool> {
    Ok(conn
        .fetch_all(sql_file!("coins/is_known_coin.sql"), vec![coin_id.into()])
        .await?
        .first()
        .ok_or(DatabaseError::RowNotFound)?
        .i64("count")?
        > 0)
}

async fn unsynced_coins(mut conn: impl SqlAccess, limit: usize) -> Result<Vec<UnsyncedCoin>> {
    let limit = i64::try_from(limit)?;

    conn.fetch_all(sql_file!("coins/unsynced_coins.sql"), vec![limit.into()])
        .await?
        .iter()
        .map(|row| {
            Ok(UnsyncedCoin {
                coin_state: CoinState::new(
                    Coin::new(
                        row.converted("parent_coin_hash")?,
                        row.converted("puzzle_hash")?,
                        row.converted("amount")?,
                    ),
                    row.opt_i64("spent_height")?.convert()?,
                    row.opt_i64("created_height")?.convert()?,
                ),
                is_asset_unsynced: row.i64("is_asset_unsynced")? != 0,
                is_children_unsynced: row.i64("is_children_unsynced")? != 0,
            })
        })
        .collect()
}

async fn delete_coin(mut conn: impl SqlAccess, coin_id: Bytes32) -> Result<()> {
    conn.execute(sql_file!("coins/delete_coin.sql"), vec![coin_id.into()])
        .await?;

    Ok(())
}

async fn update_coin(
    mut conn: impl SqlAccess,
    coin_id: Bytes32,
    asset_hash: Bytes32,
    p2_puzzle_hash: Bytes32,
) -> Result<()> {
    conn.execute(
        sql_file!("coins/update_coin.sql"),
        vec![asset_hash.into(), p2_puzzle_hash.into(), coin_id.into()],
    )
    .await?;

    Ok(())
}

async fn set_children_synced(mut conn: impl SqlAccess, coin_id: Bytes32) -> Result<()> {
    conn.execute(
        sql_file!("coins/set_children_synced.sql"),
        vec![coin_id.into()],
    )
    .await?;

    Ok(())
}

async fn set_transaction_children_unsynced(
    mut conn: impl SqlAccess,
    mempool_item_id: Bytes32,
) -> Result<()> {
    conn.execute(
        sql_file!("coins/set_transaction_children_unsynced.sql"),
        vec![mempool_item_id.into()],
    )
    .await?;

    Ok(())
}

async fn insert_lineage_proof(
    mut conn: impl SqlAccess,
    coin_id: Bytes32,
    lineage_proof: LineageProof,
) -> Result<()> {
    conn.execute(
        sql_file!("coins/insert_lineage_proof.sql"),
        vec![
            coin_id.into(),
            lineage_proof.parent_parent_coin_info.into(),
            lineage_proof.parent_inner_puzzle_hash.into(),
            lineage_proof.parent_amount.to_be_bytes().to_vec().into(),
        ],
    )
    .await?;

    Ok(())
}

async fn subscription_coin_ids(mut conn: impl SqlAccess) -> Result<Vec<Bytes32>> {
    conn.fetch_all(sql_file!("coins/subscription_coin_ids.sql"), vec![])
        .await?
        .iter()
        .map(|row| row.converted("coin_hash"))
        .collect()
}

async fn selectable_coin_count(mut conn: impl SqlAccess, asset_id: Bytes32) -> Result<u32> {
    conn.fetch_all(
        sql_file!("coins/selectable_coin_count.sql"),
        vec![asset_id.into()],
    )
    .await?
    .first()
    .ok_or(DatabaseError::RowNotFound)?
    .i64("count")?
    .convert()
}

async fn total_coin_count(mut conn: impl SqlAccess) -> Result<u32> {
    conn.fetch_all(sql_file!("coins/total_coin_count.sql"), vec![])
        .await?
        .first()
        .ok_or(DatabaseError::RowNotFound)?
        .i64("count")?
        .convert()
}

async fn synced_coin_count(mut conn: impl SqlAccess) -> Result<u32> {
    conn.fetch_all(sql_file!("coins/synced_coin_count.sql"), vec![])
        .await?
        .first()
        .ok_or(DatabaseError::RowNotFound)?
        .i64("count")?
        .convert()
}

async fn token_balance(mut conn: impl SqlAccess, asset_id: Bytes32) -> Result<u128> {
    conn.fetch_all(sql_file!("coins/token_balance.sql"), vec![asset_id.into()])
        .await?
        .iter()
        .map(|row| {
            let amount: u64 = row.converted("amount")?;
            Ok(amount as u128)
        })
        .sum()
}

async fn selectable_token_balance(mut conn: impl SqlAccess, asset_id: Bytes32) -> Result<u128> {
    conn.fetch_all(
        sql_file!("coins/selectable_token_balance.sql"),
        vec![asset_id.into()],
    )
    .await?
    .iter()
    .map(|row| {
        let amount: u64 = row.converted("amount")?;
        Ok(amount as u128)
    })
    .sum()
}

/// Decodes the coin columns shared by the `coins_by_ids` and `coin_records`
/// queries.
fn coin_row_from_row(row: &SqlRow) -> Result<CoinRow> {
    Ok(CoinRow {
        coin: Coin::new(
            row.converted("parent_coin_hash")?,
            row.converted("puzzle_hash")?,
            row.converted("amount")?,
        ),
        p2_puzzle_hash: row.converted("p2_puzzle_hash")?,
        mempool_item_hash: row.opt_converted("mempool_item_hash")?,
        offer_hash: row.opt_converted("offer_hash")?,
        kind: CoinKind::Xch,
        clawback_timestamp: row.opt_i64("clawback_timestamp")?.convert()?,
        created_height: row.opt_i64("created_height")?.convert()?,
        spent_height: row.opt_i64("spent_height")?.convert()?,
        created_timestamp: row.opt_i64("created_timestamp")?.convert()?,
        spent_timestamp: row.opt_i64("spent_timestamp")?.convert()?,
    })
}

async fn coins_by_ids(mut conn: impl SqlAccess, coin_ids: &[String]) -> Result<Vec<CoinRow>> {
    let mut sql = "
       SELECT
            parent_coin_hash, puzzle_hash, amount, spent_height, created_height, p2_puzzle_hash,
            mempool_item_hash, offer_hash, created_timestamp, spent_timestamp, clawback_expiration_seconds AS clawback_timestamp
        FROM wallet_coins
        WHERE coin_hash IN (".to_string();

    sql.push_str(&coin_hash_literals(coin_ids));
    sql.push(')');

    conn.fetch_all(&sql, vec![])
        .await?
        .iter()
        .map(coin_row_from_row)
        .collect()
}

async fn coin_records(
    mut conn: impl SqlAccess,
    asset_filter: AssetFilter,
    limit: u32,
    offset: u32,
    sort_mode: CoinSortMode,
    ascending: bool,
    filter_mode: CoinFilterMode,
) -> Result<(Vec<CoinRow>, u32)> {
    let table = match filter_mode {
        CoinFilterMode::All => "wallet_coins",
        CoinFilterMode::Selectable => "selectable_coins",
        CoinFilterMode::Owned => "owned_coins",
        CoinFilterMode::Spent => "spent_coins",
        CoinFilterMode::Clawback => "clawback_coins",
    };

    let mut sql = format!(
        "
        SELECT
            parent_coin_hash, puzzle_hash, amount,
            spent_height, created_height, p2_puzzle_hash,
            mempool_item_hash, offer_hash, created_timestamp, spent_timestamp,
            clawback_expiration_seconds AS clawback_timestamp,
            COUNT(*) OVER () AS total_count
        FROM {table}
        ",
    );

    let mut params = Vec::new();

    match asset_filter {
        AssetFilter::Id(asset_id) => {
            sql.push_str(" WHERE asset_hash = ?");
            params.push(SqlValue::from(asset_id.to_vec()));
        }
        AssetFilter::Nfts => {
            sql.push_str(" WHERE asset_kind = 1");
        }
        AssetFilter::Dids => {
            sql.push_str(" WHERE asset_kind = 2");
        }
    }

    sql.push_str(" ORDER BY ");
    sql.push_str(match sort_mode {
        CoinSortMode::CoinId if ascending => "coin_hash ASC",
        CoinSortMode::CoinId => "coin_hash DESC",
        CoinSortMode::Amount if ascending => "amount ASC",
        CoinSortMode::Amount => "amount DESC",
        CoinSortMode::CreatedHeight if ascending => "created_height ASC NULLS LAST",
        CoinSortMode::CreatedHeight => "created_height DESC NULLS FIRST",
        CoinSortMode::SpentHeight if ascending => "spent_height ASC NULLS LAST",
        CoinSortMode::SpentHeight => "spent_height DESC NULLS FIRST",
        CoinSortMode::ClawbackTimestamp if ascending => "clawback_timestamp ASC",
        CoinSortMode::ClawbackTimestamp => "clawback_timestamp DESC",
    });

    sql.push_str(" LIMIT ?");
    params.push(SqlValue::from(i64::from(limit)));
    sql.push_str(" OFFSET ?");
    params.push(SqlValue::from(i64::from(offset)));

    let rows = conn.fetch_all(&sql, params).await?;

    let total_count = rows
        .first()
        .map_or(Ok(0), |row| row.i64("total_count")?.convert())?;

    let coins = rows
        .iter()
        .map(coin_row_from_row)
        .collect::<Result<Vec<_>>>()?;

    Ok((coins, total_count))
}

async fn selectable_xch_coins(mut conn: impl SqlAccess) -> Result<Vec<Coin>> {
    conn.fetch_all(sql_file!("coins/selectable_xch_coins.sql"), vec![])
        .await?
        .iter()
        .map(|row| {
            Ok(Coin::new(
                row.converted("parent_coin_hash")?,
                row.converted("puzzle_hash")?,
                row.converted("amount")?,
            ))
        })
        .collect()
}

async fn selectable_cat_coins(mut conn: impl SqlAccess, asset_id: Bytes32) -> Result<Vec<Cat>> {
    conn.fetch_all(
        sql_file!("coins/selectable_cat_coins.sql"),
        vec![asset_id.into()],
    )
    .await?
    .iter()
    .map(|row| {
        Ok(Cat::new(
            Coin::new(
                row.converted("parent_coin_hash")?,
                row.converted("puzzle_hash")?,
                row.converted("amount")?,
            ),
            Some(LineageProof {
                parent_parent_coin_info: row.converted("parent_parent_coin_hash")?,
                parent_inner_puzzle_hash: row.converted("parent_inner_puzzle_hash")?,
                parent_amount: row.converted("parent_amount")?,
            }),
            CatInfo::new(
                asset_id,
                row.opt_converted("asset_hidden_puzzle_hash")?,
                row.converted("p2_puzzle_hash")?,
            ),
        ))
    })
    .collect()
}

async fn coin_kind(mut conn: impl SqlAccess, coin_id: Bytes32) -> Result<Option<CoinKind>> {
    let rows = conn
        .fetch_all(sql_file!("coins/coin_kind.sql"), vec![coin_id.into()])
        .await?;

    let Some(row) = rows.first() else {
        return Ok(None);
    };

    let Some(asset_id) = row.opt_i64("asset_id")? else {
        return Err(DatabaseError::InvalidEnumVariant);
    };

    let kind: AssetKind = row.i64("kind")?.convert()?;

    Ok(Some(match kind {
        AssetKind::Token => {
            if asset_id == 0 {
                CoinKind::Xch
            } else {
                CoinKind::Cat
            }
        }
        AssetKind::Nft => CoinKind::Nft,
        AssetKind::Did => CoinKind::Did,
        AssetKind::Option => CoinKind::Option,
    }))
}

async fn underlying_coin_kind(
    mut conn: impl SqlAccess,
    launcher_id: Bytes32,
) -> Result<Option<CoinKind>> {
    let rows = conn
        .fetch_all(
            sql_file!("coins/underlying_coin_kind.sql"),
            vec![launcher_id.into()],
        )
        .await?;

    let Some(row) = rows.first() else {
        return Ok(None);
    };

    let kind: AssetKind = row.i64("kind")?.convert()?;

    Ok(Some(match kind {
        AssetKind::Token => {
            if row.i64("id")? == 0 {
                CoinKind::Xch
            } else {
                CoinKind::Cat
            }
        }
        AssetKind::Nft => CoinKind::Nft,
        AssetKind::Did => CoinKind::Did,
        AssetKind::Option => CoinKind::Option,
    }))
}

async fn xch_coin(mut conn: impl SqlAccess, coin_id: Bytes32) -> Result<Option<Coin>> {
    conn.fetch_all(sql_file!("coins/xch_coin.sql"), vec![coin_id.into()])
        .await?
        .first()
        .map(|row| {
            Ok(Coin::new(
                row.converted("parent_coin_hash")?,
                row.converted("puzzle_hash")?,
                row.converted("amount")?,
            ))
        })
        .transpose()
}

async fn cat_coin(mut conn: impl SqlAccess, coin_id: Bytes32) -> Result<Option<Cat>> {
    conn.fetch_all(sql_file!("coins/cat_coin.sql"), vec![coin_id.into()])
        .await?
        .first()
        .map(|row| {
            Ok(Cat::new(
                Coin::new(
                    row.converted("parent_coin_hash")?,
                    row.converted("puzzle_hash")?,
                    row.converted("amount")?,
                ),
                Some(LineageProof {
                    parent_parent_coin_info: row.converted("parent_parent_coin_hash")?,
                    parent_inner_puzzle_hash: row.converted("parent_inner_puzzle_hash")?,
                    parent_amount: row.converted("parent_amount")?,
                }),
                CatInfo::new(
                    row.converted("asset_id")?,
                    row.opt_converted("asset_hidden_puzzle_hash")?,
                    row.converted("p2_puzzle_hash")?,
                ),
            ))
        })
        .transpose()
}

/// Decodes the columns shared by the `did_coin` and `did` queries.
fn did_from_row(row: &SqlRow) -> Result<SerializedDid> {
    Ok(SerializedDid {
        coin: Coin::new(
            row.converted("parent_coin_hash")?,
            row.converted("puzzle_hash")?,
            row.converted("amount")?,
        ),
        proof: Proof::Lineage(LineageProof {
            parent_parent_coin_info: row.converted("parent_parent_coin_hash")?,
            parent_inner_puzzle_hash: row.converted("parent_inner_puzzle_hash")?,
            parent_amount: row.converted("parent_amount")?,
        }),
        info: SerializedDidInfo {
            launcher_id: row.converted("launcher_id")?,
            recovery_list_hash: row.opt_converted("recovery_list_hash")?,
            num_verifications_required: row.i64("num_verifications_required")?.convert()?,
            metadata: row.blob("metadata")?.into(),
            p2_puzzle_hash: row.converted("p2_puzzle_hash")?,
        },
    })
}

/// Decodes the columns shared by the `nft_coin` and `nft` queries.
fn nft_from_row(row: &SqlRow) -> Result<SerializedNft> {
    Ok(SerializedNft {
        coin: Coin::new(
            row.converted("parent_coin_hash")?,
            row.converted("puzzle_hash")?,
            row.converted("amount")?,
        ),
        proof: Proof::Lineage(LineageProof {
            parent_parent_coin_info: row.converted("parent_parent_coin_hash")?,
            parent_inner_puzzle_hash: row.converted("parent_inner_puzzle_hash")?,
            parent_amount: row.converted("parent_amount")?,
        }),
        info: SerializedNftInfo {
            launcher_id: row.converted("launcher_id")?,
            metadata: row.blob("metadata")?.into(),
            metadata_updater_puzzle_hash: row.converted("metadata_updater_puzzle_hash")?,
            current_owner: row.opt_converted("owner_hash")?,
            royalty_puzzle_hash: row.converted("royalty_puzzle_hash")?,
            royalty_basis_points: row.i64("royalty_basis_points")?.convert()?,
            p2_puzzle_hash: row.converted("p2_puzzle_hash")?,
        },
    })
}

/// Decodes the columns shared by the `option_coin` and `option` queries.
fn option_from_row(row: &SqlRow) -> Result<OptionContract> {
    Ok(OptionContract::new(
        Coin::new(
            row.converted("parent_coin_hash")?,
            row.converted("puzzle_hash")?,
            row.converted("amount")?,
        ),
        Proof::Lineage(LineageProof {
            parent_parent_coin_info: row.converted("parent_parent_coin_hash")?,
            parent_inner_puzzle_hash: row.converted("parent_inner_puzzle_hash")?,
            parent_amount: row.converted("parent_amount")?,
        }),
        OptionInfo::new(
            row.converted("launcher_id")?,
            row.converted("underlying_coin_hash")?,
            row.converted("underlying_delegated_puzzle_hash")?,
            row.converted("p2_puzzle_hash")?,
        ),
    ))
}

async fn did_coin(mut conn: impl SqlAccess, coin_id: Bytes32) -> Result<Option<SerializedDid>> {
    conn.fetch_all(sql_file!("coins/did_coin.sql"), vec![coin_id.into()])
        .await?
        .first()
        .map(did_from_row)
        .transpose()
}

async fn nft_coin(mut conn: impl SqlAccess, coin_id: Bytes32) -> Result<Option<SerializedNft>> {
    conn.fetch_all(sql_file!("coins/nft_coin.sql"), vec![coin_id.into()])
        .await?
        .first()
        .map(nft_from_row)
        .transpose()
}

async fn option_coin(mut conn: impl SqlAccess, coin_id: Bytes32) -> Result<Option<OptionContract>> {
    conn.fetch_all(sql_file!("coins/option_coin.sql"), vec![coin_id.into()])
        .await?
        .first()
        .map(option_from_row)
        .transpose()
}

async fn did(mut conn: impl SqlAccess, launcher_id: Bytes32) -> Result<Option<SerializedDid>> {
    conn.fetch_all(sql_file!("coins/did.sql"), vec![launcher_id.into()])
        .await?
        .first()
        .map(did_from_row)
        .transpose()
}

async fn spendable_did(
    mut conn: impl SqlAccess,
    launcher_id: Bytes32,
) -> Result<Option<SerializedDid>> {
    conn.fetch_all(
        sql_file!("coins/spendable_did.sql"),
        vec![launcher_id.into()],
    )
    .await?
    .first()
    .map(did_from_row)
    .transpose()
}

async fn nft(mut conn: impl SqlAccess, launcher_id: Bytes32) -> Result<Option<SerializedNft>> {
    conn.fetch_all(sql_file!("coins/nft.sql"), vec![launcher_id.into()])
        .await?
        .first()
        .map(nft_from_row)
        .transpose()
}

async fn spendable_nft(
    mut conn: impl SqlAccess,
    launcher_id: Bytes32,
) -> Result<Option<SerializedNft>> {
    conn.fetch_all(
        sql_file!("coins/spendable_nft.sql"),
        vec![launcher_id.into()],
    )
    .await?
    .first()
    .map(nft_from_row)
    .transpose()
}

async fn option(mut conn: impl SqlAccess, launcher_id: Bytes32) -> Result<Option<OptionContract>> {
    conn.fetch_all(sql_file!("coins/option.sql"), vec![launcher_id.into()])
        .await?
        .first()
        .map(option_from_row)
        .transpose()
}

async fn spendable_option(
    mut conn: impl SqlAccess,
    launcher_id: Bytes32,
) -> Result<Option<OptionContract>> {
    conn.fetch_all(
        sql_file!("coins/spendable_option.sql"),
        vec![launcher_id.into()],
    )
    .await?
    .first()
    .map(option_from_row)
    .transpose()
}
