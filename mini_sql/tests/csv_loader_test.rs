use mini_sql::csv_loader::load_csv;
use mini_sql::table::DataType;

#[test]
fn test_load_csv() {
    let table = load_csv("test_data/alunos.csv")
        .expect("Falha ao carregar CSV");

    assert_eq!(table.columns.len(), 3);
    assert_eq!(table.columns[0].name, "nome");
    assert_eq!(table.columns[1].name, "idade");
    assert_eq!(table.columns[2].name, "curso");

    assert_eq!(table.columns[0].data_type, DataType::String);
    assert_eq!(table.rows.len(), 3);

    assert_eq!(
        table.rows[0],
        vec!["Ana", "20", "Computação"]
    );
}