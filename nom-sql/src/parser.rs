use std::{fmt, str};

use nom::branch::alt;
use nom::combinator::map;
use nom::IResult;
use serde::{Deserialize, Serialize};

use crate::alter::{alter_table_statement, AlterTableStatement};
use crate::compound_select::{compound_selection, CompoundSelectStatement};
use crate::create::{
    create_cached_query, creation, key_specification, view_creation, CreateCacheStatement,
    CreateTableStatement, CreateViewStatement,
};
use crate::delete::{deletion, DeleteStatement};
use crate::drop::{
    drop_cached_query, drop_table, drop_view, DropCacheStatement, DropTableStatement,
    DropViewStatement,
};
use crate::explain::{explain_statement, ExplainStatement};
use crate::insert::{insertion, InsertStatement};
use crate::rename::{rename_table, RenameTableStatement};
use crate::select::{selection, SelectStatement};
use crate::set::{set, SetStatement};
use crate::show::{show, ShowStatement};
use crate::transaction::{
    commit, rollback, start_transaction, CommitStatement, RollbackStatement,
    StartTransactionStatement,
};
use crate::update::{updating, UpdateStatement};
use crate::use_statement::{use_statement, UseStatement};
use crate::{Dialect, TableKey};

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum SqlQuery {
    CreateTable(CreateTableStatement),
    CreateView(CreateViewStatement),
    CreateCache(CreateCacheStatement),
    DropCache(DropCacheStatement),
    AlterTable(AlterTableStatement),
    Insert(InsertStatement),
    CompoundSelect(CompoundSelectStatement),
    Select(SelectStatement),
    Delete(DeleteStatement),
    DropTable(DropTableStatement),
    DropView(DropViewStatement),
    Update(UpdateStatement),
    Set(SetStatement),
    StartTransaction(StartTransactionStatement),
    Commit(CommitStatement),
    Rollback(RollbackStatement),
    RenameTable(RenameTableStatement),
    Use(UseStatement),
    Show(ShowStatement),
    Explain(ExplainStatement),
}

impl fmt::Display for SqlQuery {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SqlQuery::Select(ref select) => write!(f, "{}", select),
            SqlQuery::Insert(ref insert) => write!(f, "{}", insert),
            SqlQuery::CreateTable(ref create) => write!(f, "{}", create),
            SqlQuery::CreateView(ref create) => write!(f, "{}", create),
            SqlQuery::CreateCache(ref create) => write!(f, "{}", create),
            SqlQuery::DropCache(ref drop) => write!(f, "{}", drop),
            SqlQuery::Delete(ref delete) => write!(f, "{}", delete),
            SqlQuery::DropTable(ref drop) => write!(f, "{}", drop),
            SqlQuery::DropView(ref drop) => write!(f, "{}", drop),
            SqlQuery::Update(ref update) => write!(f, "{}", update),
            SqlQuery::Set(ref set) => write!(f, "{}", set),
            SqlQuery::AlterTable(ref alter) => write!(f, "{}", alter),
            SqlQuery::CompoundSelect(ref compound) => write!(f, "{}", compound),
            SqlQuery::StartTransaction(ref tx) => write!(f, "{}", tx),
            SqlQuery::Commit(ref commit) => write!(f, "{}", commit),
            SqlQuery::Rollback(ref rollback) => write!(f, "{}", rollback),
            SqlQuery::RenameTable(ref rename) => write!(f, "{}", rename),
            SqlQuery::Use(ref use_db) => write!(f, "{}", use_db),
            SqlQuery::Show(ref show) => write!(f, "{}", show),
            SqlQuery::Explain(ref explain) => write!(f, "{}", explain),
        }
    }
}

impl str::FromStr for SqlQuery {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_query(Dialect::MySQL, s)
    }
}

impl SqlQuery {
    /// Returns the type of the query, e.g. "CREATE TABLE" or "SELECT"
    pub fn query_type(&self) -> &'static str {
        match self {
            Self::Select(_) => "SELECT",
            Self::Insert(_) => "INSERT",
            Self::CreateTable(_) => "CREATE TABLE",
            Self::CreateView(_) => "CREATE VIEW",
            Self::CreateCache(_) => "CREATE CACHE",
            Self::DropCache(_) => "DROP CACHE",
            Self::Delete(_) => "DELETE",
            Self::DropTable(_) => "DROP TABLE",
            Self::DropView(_) => "DROP VIEW",
            Self::Update(_) => "UPDATE",
            Self::Set(_) => "SET",
            Self::AlterTable(_) => "ALTER TABLE",
            Self::CompoundSelect(_) => "SELECT",
            Self::StartTransaction(_) => "START TRANSACTION",
            Self::Commit(_) => "COMMIT",
            Self::Rollback(_) => "ROLLBACK",
            Self::RenameTable(_) => "RENAME",
            Self::Use(_) => "USE",
            Self::Show(_) => "SHOW",
            Self::Explain(_) => "EXPLAIN",
        }
    }
}

pub fn sql_query(dialect: Dialect) -> impl Fn(&[u8]) -> IResult<&[u8], SqlQuery> {
    move |i| {
        alt((
            map(creation(dialect), SqlQuery::CreateTable),
            map(insertion(dialect), SqlQuery::Insert),
            map(compound_selection(dialect), SqlQuery::CompoundSelect),
            map(selection(dialect), SqlQuery::Select),
            map(deletion(dialect), SqlQuery::Delete),
            map(drop_table(dialect), SqlQuery::DropTable),
            map(drop_view(dialect), SqlQuery::DropView),
            map(updating(dialect), SqlQuery::Update),
            map(set(dialect), SqlQuery::Set),
            map(view_creation(dialect), SqlQuery::CreateView),
            map(create_cached_query(dialect), SqlQuery::CreateCache),
            map(drop_cached_query(dialect), SqlQuery::DropCache),
            map(alter_table_statement(dialect), SqlQuery::AlterTable),
            map(start_transaction(dialect), SqlQuery::StartTransaction),
            map(commit(dialect), SqlQuery::Commit),
            map(rollback(dialect), SqlQuery::Rollback),
            map(rename_table(dialect), SqlQuery::RenameTable),
            map(use_statement(dialect), SqlQuery::Use),
            map(show(dialect), SqlQuery::Show),
            map(explain_statement, SqlQuery::Explain),
        ))(i)
    }
}

/// Parse a SQL query from a byte slice
pub fn parse_query_bytes<T>(dialect: Dialect, input: T) -> Result<SqlQuery, &'static str>
where
    T: AsRef<[u8]>,
{
    match sql_query(dialect)(input.as_ref()) {
        Ok((_, o)) => Ok(o),
        Err(_) => Err("failed to parse query"),
    }
}

/// Parse a SQL query from a string
// TODO(fran): Make this function return a ReadySetResult.
pub fn parse_query<T>(dialect: Dialect, input: T) -> Result<SqlQuery, &'static str>
where
    T: AsRef<str>,
{
    parse_query_bytes(dialect, input.as_ref().trim().as_bytes())
}

/// Parse a select statement from a byte slice
pub fn parse_select_statement_bytes<T>(
    dialect: Dialect,
    input: T,
) -> Result<SelectStatement, &'static str>
where
    T: AsRef<[u8]>,
{
    match selection(dialect)(input.as_ref()) {
        Ok((remaining, o)) if remaining.is_empty() => Ok(o),
        _ => Err("failed to parse query"),
    }
}

/// Parse a select statement from a string
pub fn parse_select_statement<T>(
    dialect: Dialect,
    input: T,
) -> Result<SelectStatement, &'static str>
where
    T: AsRef<str>,
{
    parse_select_statement_bytes(dialect, input.as_ref().trim().as_bytes())
}

/// Parse a create table statement from a byte slice
pub fn parse_create_table_bytes<T>(
    dialect: Dialect,
    input: T,
) -> Result<CreateTableStatement, &'static str>
where
    T: AsRef<[u8]>,
{
    match creation(dialect)(input.as_ref()) {
        Ok((remaining, o)) if remaining.is_empty() => Ok(o),
        _ => Err("failed to parse query"),
    }
}

/// Parse a create table statement from a string
pub fn parse_create_table<T>(
    dialect: Dialect,
    input: T,
) -> Result<CreateTableStatement, &'static str>
where
    T: AsRef<str>,
{
    parse_create_table_bytes(dialect, input.as_ref().trim().as_bytes())
}

/// Parse an alter table statement from a byte slice
pub fn parse_alter_table_bytes<T>(
    dialect: Dialect,
    input: T,
) -> Result<AlterTableStatement, &'static str>
where
    T: AsRef<[u8]>,
{
    match alter_table_statement(dialect)(input.as_ref()) {
        Ok((remaining, o)) if remaining.is_empty() => Ok(o),
        _ => Err("failed to parse query"),
    }
}

/// Parse an alter table statement from a string
pub fn parse_alter_table<T>(dialect: Dialect, input: T) -> Result<AlterTableStatement, &'static str>
where
    T: AsRef<str>,
{
    parse_alter_table_bytes(dialect, input.as_ref().trim().as_bytes())
}

/// Parse a specification for a table key or constraint from a byte slice
pub fn parse_key_specification_bytes<T>(
    dialect: Dialect,
    input: T,
) -> Result<TableKey, &'static str>
where
    T: AsRef<[u8]>,
{
    match key_specification(dialect)(input.as_ref()) {
        Ok((_, o)) => Ok(o),
        Err(_) => Err("failed to parse query"),
    }
}

/// Parse a specification for a table key or constraint from a string
pub fn parse_key_specification_string<T>(
    dialect: Dialect,
    input: T,
) -> Result<TableKey, &'static str>
where
    T: AsRef<str>,
{
    parse_key_specification_bytes(dialect, input.as_ref().trim().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_select_query() {
        let qstring0 = "SELECT * FROM `users`";
        let qstring1 = "SELECT * FROM `users` AS `u`";
        let qstring2 = "SELECT `name`, `password` FROM `users` AS `u`";
        let qstring3 = "SELECT `name`, `password` FROM `users` AS `u` WHERE (`user_id` = '1')";
        let qstring4 =
            "SELECT `name`, `password` FROM `users` AS `u` WHERE ((`user` = 'aaa') AND (`password` = 'xxx'))";
        let qstring5 = "SELECT (`name` * 2) AS `double_name` FROM `users`";

        let res0 = parse_query(Dialect::MySQL, qstring0);
        let res1 = parse_query(Dialect::MySQL, qstring1);
        let res2 = parse_query(Dialect::MySQL, qstring2);
        let res3 = parse_query(Dialect::MySQL, qstring3);
        let res4 = parse_query(Dialect::MySQL, qstring4);
        let res5 = parse_query(Dialect::MySQL, qstring5);

        assert!(res0.is_ok());
        assert!(res1.is_ok());
        assert!(res2.is_ok());
        assert!(res3.is_ok());
        assert!(res4.is_ok());
        assert!(res5.is_ok());

        assert_eq!(qstring0, format!("{}", res0.unwrap()));
        assert_eq!(qstring1, format!("{}", res1.unwrap()));
        assert_eq!(qstring2, format!("{}", res2.unwrap()));
        assert_eq!(qstring3, format!("{}", res3.unwrap()));
        assert_eq!(qstring4, format!("{}", res4.unwrap()));
        assert_eq!(qstring5, format!("{}", res5.unwrap()));
    }

    #[test]
    fn format_select_query() {
        let qstring1 = "select * from users u";
        let qstring2 = "select name,password from users u;";
        let qstring3 = "select name,password from users u WHERE user_id='1'";

        let expected1 = "SELECT * FROM `users` AS `u`";
        let expected2 = "SELECT `name`, `password` FROM `users` AS `u`";
        let expected3 = "SELECT `name`, `password` FROM `users` AS `u` WHERE (`user_id` = '1')";

        let res1 = parse_query(Dialect::MySQL, qstring1);
        let res2 = parse_query(Dialect::MySQL, qstring2);
        let res3 = parse_query(Dialect::MySQL, qstring3);

        assert!(res1.is_ok());
        assert!(res2.is_ok());
        assert!(res3.is_ok());

        assert_eq!(expected1, format!("{}", res1.unwrap()));
        assert_eq!(expected2, format!("{}", res2.unwrap()));
        assert_eq!(expected3, format!("{}", res3.unwrap()));
    }

    #[test]
    fn format_select_query_with_where_clause() {
        let qstring0 = "select name, password from users as u where user='aaa' and password= 'xxx'";
        let qstring1 = "select name, password from users as u where user=? and password =?";

        let expected0 =
            "SELECT `name`, `password` FROM `users` AS `u` WHERE ((`user` = 'aaa') AND (`password` = 'xxx'))";
        let expected1 =
            "SELECT `name`, `password` FROM `users` AS `u` WHERE ((`user` = ?) AND (`password` = ?))";

        let res0 = parse_query(Dialect::MySQL, qstring0);
        let res1 = parse_query(Dialect::MySQL, qstring1);
        assert!(res0.is_ok());
        assert!(res1.is_ok());
        assert_eq!(expected0, format!("{}", res0.unwrap()));
        assert_eq!(expected1, format!("{}", res1.unwrap()));
    }

    #[test]
    fn format_select_query_with_function() {
        let qstring1 = "select count(*) from users";
        let expected1 = "SELECT count(*) FROM `users`";

        let res1 = parse_query(Dialect::MySQL, qstring1);
        assert!(res1.is_ok());
        assert_eq!(expected1, format!("{}", res1.unwrap()));
    }

    #[test]
    fn display_insert_query() {
        let qstring = "INSERT INTO users (name, password) VALUES ('aaa', 'xxx')";
        let expected = "INSERT INTO `users` (`name`, `password`) VALUES ('aaa', 'xxx')";
        let res = parse_query(Dialect::MySQL, qstring);
        assert!(res.is_ok());
        assert_eq!(expected, format!("{}", res.unwrap()));
    }

    #[test]
    fn display_insert_query_no_columns() {
        let qstring = "INSERT INTO users VALUES ('aaa', 'xxx')";
        let expected = "INSERT INTO `users` VALUES ('aaa', 'xxx')";
        let res = parse_query(Dialect::MySQL, qstring);
        assert!(res.is_ok());
        assert_eq!(expected, format!("{}", res.unwrap()));
    }

    #[test]
    fn format_insert_query() {
        let qstring = "insert into users (name, password) values ('aaa', 'xxx')";
        let expected = "INSERT INTO `users` (`name`, `password`) VALUES ('aaa', 'xxx')";
        let res = parse_query(Dialect::MySQL, qstring);
        assert!(res.is_ok());
        assert_eq!(expected, format!("{}", res.unwrap()));
    }

    #[test]
    fn format_update_query() {
        let qstring = "update users set name=42, password='xxx' where id=1";
        let expected = "UPDATE `users` SET `name` = 42, `password` = 'xxx' WHERE (`id` = 1)";
        let res = parse_query(Dialect::MySQL, qstring);
        assert!(res.is_ok());
        assert_eq!(expected, format!("{}", res.unwrap()));
    }

    #[test]
    fn format_delete_query_with_where_clause() {
        let qstring0 = "delete from users where user='aaa' and password= 'xxx'";
        let qstring1 = "delete from users where user=? and password =?";

        let expected0 = "DELETE FROM `users` WHERE ((`user` = 'aaa') AND (`password` = 'xxx'))";
        let expected1 = "DELETE FROM `users` WHERE ((`user` = ?) AND (`password` = ?))";

        let res0 = parse_query(Dialect::MySQL, qstring0);
        let res1 = parse_query(Dialect::MySQL, qstring1);
        assert!(res0.is_ok());
        assert!(res1.is_ok());
        assert_eq!(expected0, format!("{}", res0.unwrap()));
        assert_eq!(expected1, format!("{}", res1.unwrap()));
    }

    mod mysql {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        use super::*;
        use crate::table::Table;

        #[test]
        fn trim_query() {
            let qstring = "   INSERT INTO users VALUES (42, \"test\");     ";
            let res = parse_query(Dialect::MySQL, qstring);
            assert!(res.is_ok());
        }

        #[test]
        fn parse_byte_slice() {
            let qstring: &[u8] = b"INSERT INTO users VALUES (42, \"test\");";
            let res = parse_query_bytes(Dialect::MySQL, qstring);
            assert!(res.is_ok());
        }

        #[test]
        fn parse_byte_vector() {
            let qstring: Vec<u8> = b"INSERT INTO users VALUES (42, \"test\");".to_vec();
            let res = parse_query_bytes(Dialect::MySQL, &qstring);
            assert!(res.is_ok());
        }

        #[test]
        fn hash_query() {
            let qstring = "INSERT INTO users VALUES (42, \"test\");";
            let res = parse_query(Dialect::MySQL, qstring);
            assert!(res.is_ok());

            let expected = SqlQuery::Insert(InsertStatement {
                table: Table::from("users"),
                fields: None,
                data: vec![vec![42.into(), "test".into()]],
                ..Default::default()
            });
            let mut h0 = DefaultHasher::new();
            let mut h1 = DefaultHasher::new();
            res.unwrap().hash(&mut h0);
            expected.hash(&mut h1);
            assert_eq!(h0.finish(), h1.finish());
        }

        #[test]
        fn format_query_with_escaped_keyword() {
            let qstring0 = "delete from articles where `key`='aaa'";
            let qstring1 = "delete from `where` where user=?";

            let expected0 = "DELETE FROM `articles` WHERE (`key` = 'aaa')";
            let expected1 = "DELETE FROM `where` WHERE (`user` = ?)";

            let res0 = parse_query(Dialect::MySQL, qstring0);
            let res1 = parse_query(Dialect::MySQL, qstring1);
            assert!(res0.is_ok());
            assert!(res1.is_ok());
            assert_eq!(expected0, format!("{}", res0.unwrap()));
            assert_eq!(expected1, format!("{}", res1.unwrap()));
        }
    }

    mod tests_postgres {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        use super::*;
        use crate::table::Table;

        #[test]
        fn trim_query() {
            let qstring = "   INSERT INTO users VALUES (42, 'test');     ";
            let res = parse_query(Dialect::PostgreSQL, qstring);
            assert!(res.is_ok());
        }

        #[test]
        fn parse_byte_slice() {
            let qstring: &[u8] = b"INSERT INTO users VALUES (42, 'test');";
            let res = parse_query_bytes(Dialect::PostgreSQL, qstring);
            assert!(res.is_ok());
        }

        #[test]
        fn parse_byte_vector() {
            let qstring: Vec<u8> = b"INSERT INTO users VALUES (42, 'test');".to_vec();
            let res = parse_query_bytes(Dialect::PostgreSQL, &qstring);
            assert!(res.is_ok());
        }

        #[test]
        fn hash_query() {
            let qstring = "INSERT INTO users VALUES (42, 'test');";
            let res = parse_query(Dialect::PostgreSQL, qstring);
            assert!(res.is_ok());

            let expected = SqlQuery::Insert(InsertStatement {
                table: Table::from("users"),
                fields: None,
                data: vec![vec![42.into(), "test".into()]],
                ..Default::default()
            });
            let mut h0 = DefaultHasher::new();
            let mut h1 = DefaultHasher::new();
            res.unwrap().hash(&mut h0);
            expected.hash(&mut h1);
            assert_eq!(h0.finish(), h1.finish());
        }

        #[test]
        fn format_query_with_escaped_keyword() {
            let qstring0 = "delete from articles where \"key\"='aaa'";
            let qstring1 = "delete from \"where\" where user=?";

            let expected0 = "DELETE FROM `articles` WHERE (`key` = 'aaa')";
            let expected1 = "DELETE FROM `where` WHERE (`user` = ?)";

            let res0 = parse_query(Dialect::PostgreSQL, qstring0);
            let res1 = parse_query(Dialect::PostgreSQL, qstring1);
            assert!(res0.is_ok());
            assert!(res1.is_ok());
            assert_eq!(expected0, format!("{}", res0.unwrap()));
            assert_eq!(expected1, format!("{}", res1.unwrap()));
        }
    }
}
