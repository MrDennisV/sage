use chia_protocol::{Bytes32, Coin, Program};

use crate::{
    Asset, AssetKind, CoinKind, CoinRow, Convert, Database, DatabaseError, DatabaseTx, Result,
    SqlAccess, SqlExecutor, SqlValue, sql_file,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NftSortMode {
    Recent,
    Name,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NftGroupSearch {
    Collection(Bytes32),
    NoCollection,
    MinterDid(Bytes32),
    NoMinterDid,
    OwnerDid(Bytes32),
    NoOwnerDid,
}

#[derive(Debug, Clone)]
pub struct NftCoinInfo {
    pub collection_hash: Bytes32,
    pub collection_name: Option<String>,
    pub minter_hash: Option<Bytes32>,
    pub owner_hash: Option<Bytes32>,
    pub metadata: Program,
    pub metadata_updater_puzzle_hash: Bytes32,
    pub royalty_puzzle_hash: Bytes32,
    pub royalty_basis_points: u16,
    pub data_hash: Option<Bytes32>,
    pub metadata_hash: Option<Bytes32>,
    pub license_hash: Option<Bytes32>,
    pub edition_number: Option<u64>,
    pub edition_total: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct NftRow {
    pub asset: Asset,
    pub nft_info: NftCoinInfo,
    pub coin_row: CoinRow,
}

#[derive(Debug, Clone)]
pub struct NftMetadataInfo {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_sensitive_content: bool,
    pub collection_id: Bytes32,
}

#[derive(Debug, Clone)]
pub struct NftOfferInfo {
    pub metadata: Program,
    pub metadata_updater_puzzle_hash: Bytes32,
    pub royalty_puzzle_hash: Bytes32,
    pub royalty_basis_points: u16,
}

impl<E: SqlExecutor> Database<E> {
    pub async fn wallet_nft(&self, hash: Bytes32) -> Result<Option<NftRow>> {
        wallet_nft(&self.executor, hash).await
    }

    pub async fn owned_nfts(
        &self,
        name_search: Option<String>,
        group_search: Option<NftGroupSearch>,
        sort_mode: NftSortMode,
        include_hidden: bool,
        limit: u32,
        offset: u32,
    ) -> Result<(Vec<NftRow>, u32)> {
        owned_nfts(
            &self.executor,
            name_search,
            group_search,
            sort_mode,
            include_hidden,
            limit,
            offset,
        )
        .await
    }

    pub async fn distinct_minter_dids(
        &self,
        limit: u32,
        offset: u32,
    ) -> Result<(Vec<Bytes32>, u32)> {
        distinct_minter_dids(&self.executor, limit, offset).await
    }

    pub async fn offer_nft_info(&self, hash: Bytes32) -> Result<Option<NftOfferInfo>> {
        offer_nft_info(&self.executor, hash).await
    }
}

async fn wallet_nft(mut conn: impl SqlAccess, hash: Bytes32) -> Result<Option<NftRow>> {
    conn.fetch_all(sql_file!("nfts/wallet_nft.sql"), vec![hash.into()])
        .await?
        .first()
        .map(|row| {
            // The `?` suffixes are part of the column aliases in the query, where
            // they mark the columns sqlx should treat as nullable.
            Ok(NftRow {
                asset: Asset {
                    hash: row.converted("asset_hash")?,
                    name: row.opt_text("asset_name")?,
                    ticker: row.opt_text("asset_ticker")?,
                    precision: row.i64("asset_precision")?.convert()?,
                    icon_url: row.opt_text("asset_icon_url")?,
                    description: row.opt_text("asset_description")?,
                    is_sensitive_content: row.i64("asset_is_sensitive_content")? != 0,
                    is_visible: row.i64("asset_is_visible")? != 0,
                    hidden_puzzle_hash: row.opt_converted("asset_hidden_puzzle_hash")?,
                    kind: AssetKind::Nft,
                },
                nft_info: NftCoinInfo {
                    collection_hash: row.opt_converted("collection_hash?")?.unwrap_or_default(),
                    collection_name: row.opt_text("collection_name")?,
                    minter_hash: row.opt_converted("minter_hash")?,
                    owner_hash: row.opt_converted("owner_hash")?,
                    metadata: row.blob("metadata")?.into(),
                    metadata_updater_puzzle_hash: row.converted("metadata_updater_puzzle_hash")?,
                    royalty_puzzle_hash: row.converted("royalty_puzzle_hash")?,
                    royalty_basis_points: row.i64("royalty_basis_points")?.convert()?,
                    data_hash: row.opt_converted("data_hash")?,
                    metadata_hash: row.opt_converted("metadata_hash")?,
                    license_hash: row.opt_converted("license_hash")?,
                    edition_number: row.opt_i64("edition_number")?.convert()?,
                    edition_total: row.opt_i64("edition_total")?.convert()?,
                },
                coin_row: CoinRow {
                    coin: Coin::new(
                        row.converted("parent_coin_hash")?,
                        row.converted("puzzle_hash")?,
                        row.converted("amount")?,
                    ),
                    p2_puzzle_hash: row.converted("p2_puzzle_hash")?,
                    kind: CoinKind::Nft,
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

async fn owned_nfts(
    mut conn: impl SqlAccess,
    name_search: Option<String>,
    group_search: Option<NftGroupSearch>,
    sort_mode: NftSortMode,
    include_hidden: bool,
    limit: u32,
    offset: u32,
) -> Result<(Vec<NftRow>, u32)> {
    let mut sql = "
            SELECT
                asset_hash, asset_name, asset_ticker, asset_precision, asset_icon_url,
                asset_description, asset_is_sensitive_content, asset_hidden_puzzle_hash,
                asset_is_visible AND (collections.id IS NULL OR collections.is_visible) as is_visible,
                collections.hash AS collection_hash, collections.name AS collection_name,
                owned_nfts.minter_hash, owner_hash, metadata, metadata_updater_puzzle_hash,
                royalty_puzzle_hash, royalty_basis_points, data_hash, metadata_hash, license_hash,
                parent_coin_hash, puzzle_hash, amount, p2_puzzle_hash, edition_number, edition_total,
                created_height, spent_height, offer_hash, created_timestamp, spent_timestamp,
                clawback_expiration_seconds AS clawback_timestamp, COUNT(*) OVER() as total_count
            FROM owned_nfts
            LEFT JOIN collections ON collections.id = owned_nfts.collection_id
            WHERE 1=1
            "
    .to_string();

    let mut params = Vec::new();

    if !include_hidden {
        sql.push_str("AND (asset_is_visible = 1 AND (owned_nfts.collection_id IS NULL OR collections.is_visible = 1))");
    }

    if let Some(name_search) = name_search {
        sql.push_str("AND asset_name LIKE ?");
        params.push(SqlValue::from(format!("%{name_search}%")));
    }

    if let Some(group) = group_search {
        match group {
            NftGroupSearch::Collection(id) => {
                sql.push_str(" AND collections.hash = ?");
                params.push(SqlValue::from(id.as_ref().to_vec()));
            }
            NftGroupSearch::NoCollection => {
                sql.push_str(" AND collections.hash IS NULL");
            }
            NftGroupSearch::MinterDid(id) => {
                sql.push_str(" AND owned_nfts.minter_hash = ?");
                params.push(SqlValue::from(id.as_ref().to_vec()));
            }
            NftGroupSearch::NoMinterDid => {
                sql.push_str(" AND owned_nfts.minter_hash IS NULL");
            }
            NftGroupSearch::OwnerDid(id) => {
                sql.push_str(" AND owned_nfts.owner_hash = ?");
                params.push(SqlValue::from(id.as_ref().to_vec()));
            }
            NftGroupSearch::NoOwnerDid => {
                sql.push_str(" AND owner_hash IS NULL");
            }
        }
    }

    // Add ORDER BY clause based on sort_mode
    sql.push_str(" ORDER BY ");

    match sort_mode {
        NftSortMode::Recent => {
            sql.push_str("(created_height IS NULL) DESC, created_height DESC");
        }
        NftSortMode::Name => {
            sql.push_str("(asset_name IS NULL) ASC, asset_name ASC, edition_number ASC");
        }
    }

    sql.push_str(" LIMIT ?");
    params.push(SqlValue::from(limit));
    sql.push_str(" OFFSET ?");
    params.push(SqlValue::from(offset));

    let rows = conn.fetch_all(&sql, params).await?;

    let total_count = rows
        .first()
        .map_or(Ok(0), |row| row.i64("total_count")?.convert())?;

    let nfts = rows
        .iter()
        .map(|row| {
            Ok(NftRow {
                asset: Asset {
                    hash: row.converted("asset_hash")?,
                    name: row.opt_text("asset_name")?,
                    ticker: row.opt_text("asset_ticker")?,
                    precision: row.i64("asset_precision")?.convert()?,
                    icon_url: row.opt_text("asset_icon_url")?,
                    description: row.opt_text("asset_description")?,
                    is_visible: row.i64("is_visible")? != 0,
                    is_sensitive_content: row.i64("asset_is_sensitive_content")? != 0,
                    hidden_puzzle_hash: row.opt_converted("asset_hidden_puzzle_hash")?,
                    kind: AssetKind::Nft,
                },
                nft_info: NftCoinInfo {
                    collection_hash: row.opt_converted("collection_hash")?.unwrap_or_default(),
                    collection_name: row.opt_text("collection_name")?,
                    minter_hash: row.opt_converted("minter_hash")?,
                    owner_hash: row.opt_converted("owner_hash")?,
                    metadata: row.blob("metadata")?.into(),
                    metadata_updater_puzzle_hash: row.converted("metadata_updater_puzzle_hash")?,
                    royalty_puzzle_hash: row.converted("royalty_puzzle_hash")?,
                    royalty_basis_points: row.i64("royalty_basis_points")?.convert()?,
                    data_hash: row.opt_converted("data_hash")?,
                    metadata_hash: row.opt_converted("metadata_hash")?,
                    license_hash: row.opt_converted("license_hash")?,
                    edition_number: row.opt_i64("edition_number")?.convert()?,
                    edition_total: row.opt_i64("edition_total")?.convert()?,
                },
                coin_row: CoinRow {
                    coin: Coin::new(
                        row.converted("parent_coin_hash")?,
                        row.converted("puzzle_hash")?,
                        row.converted("amount")?,
                    ),
                    p2_puzzle_hash: row.converted("p2_puzzle_hash")?,
                    kind: CoinKind::Nft,
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
        .collect::<Result<Vec<NftRow>>>()?;

    Ok((nfts, total_count))
}

async fn distinct_minter_dids(
    mut conn: impl SqlAccess,
    limit: u32,
    offset: u32,
) -> Result<(Vec<Bytes32>, u32)> {
    let rows = conn
        .fetch_all(
            sql_file!("nfts/distinct_minter_dids.sql"),
            vec![limit.into(), offset.into()],
        )
        .await?;

    let total_count = conn
        .fetch_all(sql_file!("nfts/distinct_minter_did_count.sql"), vec![])
        .await?
        .first()
        .ok_or(DatabaseError::RowNotFound)?
        .i64("total_count")?
        .try_into()
        .map_err(DatabaseError::PrecisionLost)?;

    let mut dids = Vec::new();

    for row in &rows {
        if let Some(minter_hash) = row.opt_blob("minter_hash")? {
            dids.push(minter_hash.convert()?);
        }
    }

    Ok((dids, total_count))
}

impl<E: SqlExecutor> DatabaseTx<'_, E> {
    pub async fn insert_nft(&mut self, hash: Bytes32, coin_info: &NftCoinInfo) -> Result<()> {
        insert_nft(&mut self.tx, hash, coin_info).await
    }

    pub async fn update_nft(&mut self, hash: Bytes32, coin_info: &NftCoinInfo) -> Result<()> {
        update_nft(&mut self.tx, hash, coin_info).await
    }

    pub async fn update_nft_data_hash_urls(
        &mut self,
        data_hash: Bytes32,
        icon_url: String,
    ) -> Result<()> {
        update_nft_data_hash_urls(&mut self.tx, data_hash, icon_url).await
    }

    pub async fn update_nft_metadata(
        &mut self,
        hash: Bytes32,
        metadata_info: NftMetadataInfo,
    ) -> Result<()> {
        update_nft_metadata(&mut self.tx, hash, metadata_info).await
    }
}

async fn update_nft_data_hash_urls(
    mut conn: impl SqlAccess,
    data_hash: Bytes32,
    icon_url: String,
) -> Result<()> {
    conn.execute(
        sql_file!("nfts/update_nft_data_hash_urls.sql"),
        vec![icon_url.into(), data_hash.into()],
    )
    .await?;

    Ok(())
}

async fn update_nft_metadata(
    mut conn: impl SqlAccess,
    hash: Bytes32,
    metadata_info: NftMetadataInfo,
) -> Result<()> {
    conn.execute(
        sql_file!("nfts/update_nft_metadata_asset.sql"),
        vec![
            metadata_info.name.into(),
            metadata_info.description.into(),
            metadata_info.is_sensitive_content.into(),
            hash.into(),
        ],
    )
    .await?;

    conn.execute(
        sql_file!("nfts/update_nft_metadata_collection.sql"),
        vec![metadata_info.collection_id.into(), hash.into()],
    )
    .await?;

    Ok(())
}

async fn insert_nft(
    mut conn: impl SqlAccess,
    hash: Bytes32,
    coin_info: &NftCoinInfo,
) -> Result<()> {
    let edition_number: Option<i64> = coin_info
        .edition_number
        .map(TryInto::try_into)
        .transpose()?;
    let edition_total: Option<i64> = coin_info.edition_total.map(TryInto::try_into).transpose()?;

    conn.execute(
        sql_file!("nfts/insert_nft.sql"),
        vec![
            hash.into(),
            coin_info.collection_hash.into(),
            coin_info.minter_hash.into(),
            coin_info.owner_hash.into(),
            coin_info.metadata.as_slice().into(),
            coin_info.metadata_updater_puzzle_hash.into(),
            coin_info.royalty_puzzle_hash.into(),
            coin_info.royalty_basis_points.into(),
            coin_info.data_hash.into(),
            coin_info.metadata_hash.into(),
            coin_info.license_hash.into(),
            edition_number.into(),
            edition_total.into(),
        ],
    )
    .await?;

    Ok(())
}

async fn update_nft(
    mut conn: impl SqlAccess,
    hash: Bytes32,
    coin_info: &NftCoinInfo,
) -> Result<()> {
    let edition_number: Option<i64> = coin_info
        .edition_number
        .map(TryInto::try_into)
        .transpose()?;
    let edition_total: Option<i64> = coin_info.edition_total.map(TryInto::try_into).transpose()?;

    conn.execute(
        sql_file!("nfts/update_nft.sql"),
        vec![
            coin_info.collection_hash.into(),
            coin_info.minter_hash.into(),
            coin_info.owner_hash.into(),
            coin_info.metadata.as_slice().into(),
            coin_info.metadata_updater_puzzle_hash.into(),
            coin_info.royalty_puzzle_hash.into(),
            coin_info.royalty_basis_points.into(),
            coin_info.data_hash.into(),
            coin_info.metadata_hash.into(),
            coin_info.license_hash.into(),
            edition_number.into(),
            edition_total.into(),
            hash.into(),
        ],
    )
    .await?;

    Ok(())
}

async fn offer_nft_info(mut conn: impl SqlAccess, hash: Bytes32) -> Result<Option<NftOfferInfo>> {
    conn.fetch_all(sql_file!("nfts/offer_nft_info.sql"), vec![hash.into()])
        .await?
        .first()
        .map(|row| {
            Ok(NftOfferInfo {
                metadata: Program::from(row.blob("metadata")?),
                metadata_updater_puzzle_hash: row.converted("metadata_updater_puzzle_hash")?,
                royalty_puzzle_hash: row.converted("royalty_puzzle_hash")?,
                royalty_basis_points: row.i64("royalty_basis_points")?.convert()?,
            })
        })
        .transpose()
}
