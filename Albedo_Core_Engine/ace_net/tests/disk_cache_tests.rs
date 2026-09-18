use ace_net::cache::disk::{DiskCacheEngine, SparseRangeIndex};
use bytes::Bytes;
use tempfile::tempdir;

#[test]
fn test_sparse_range_coalescence() {
    let mut index = SparseRangeIndex::new();
    
    // 1. Inserir faixas fora de ordem
    index.add_range(100, 199);
    index.add_range(0, 99);
    assert_eq!(index.ranges, vec![(0, 199)]);

    // 2. Inserir faixa sobreposta
    index.add_range(150, 250);
    assert_eq!(index.ranges, vec![(0, 250)]);

    // 3. Inserir faixa disjunta
    index.add_range(300, 399);
    assert_eq!(index.ranges, vec![(0, 250), (300, 399)]);

    // 4. Inserir faixa que liga duas disjuntas
    index.add_range(251, 299);
    assert_eq!(index.ranges, vec![(0, 399)]);
}

#[tokio::test]
async fn test_binary_index_recovery() {
    let dir = tempdir().unwrap();
    let path = dir.path().to_path_buf();

    // Inicializa o motor
    let engine = DiskCacheEngine::new(path.clone(), 1024 * 1024).await.unwrap();

    // 1. Grava dados esparsos e totais no engine
    engine.put_range(0x1234, 0, 99, Some(200), Bytes::from_static(&[1u8; 100])).await.unwrap();
    
    // 2. Grava um item completo no WAL via put_range (simulando cacheamento)
    engine.put_range(0x5678, 0, 49, None, Bytes::from_static(&[2u8; 50])).await.unwrap();

    // 3. Força o dump do índice e limpa WAL
    engine.save_index().await.unwrap();

    // Simula a reinicialização lendo do cache.idx
    let engine_recovered = DiskCacheEngine::new(path.clone(), 1024 * 1024).await.unwrap();

    // Verifica se os ranges e metadados persistem via índice binário
    let bytes1 = engine_recovered.get_range(0x1234, 0, 99).await.unwrap();
    assert_eq!(bytes1.len(), 100);
    assert_eq!(bytes1[0], 1u8);

    let bytes2 = engine_recovered.get_range(0x5678, 0, 49).await.unwrap();
    assert_eq!(bytes2.len(), 50);
    assert_eq!(bytes2[0], 2u8);
}
