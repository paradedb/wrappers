-- create foreign data wrapper and enable 'ClickHouseFdw'
drop foreign data wrapper if exists clickhouse_wrapper;
create foreign data wrapper clickhouse_wrapper
  handler wrappers_handler
  validator wrappers_validator
  options (
    wrapper 'ClickHouseFdw'
  );

-- create and save ClickHouse connection string in Vault
select (pgsodium.create_key(name := 'clickhouse')).name;
insert into vault.secrets (secret, key_id) values (
  'tcp://default:@clickhouse:9000/supa',
  (select id from pgsodium.valid_key where name = 'clickhouse')
);

-- create a wrappers ClickHouse server with connection string id option
do $$
declare
  csid text;
begin
  select id into csid from pgsodium.valid_key where name = 'clickhouse' limit 1;

  drop server if exists my_clickhouse_server cascade;

  execute format(
    E'create server my_clickhouse_server \n'
    '   foreign data wrapper clickhouse_wrapper \n'
    '   options (conn_string_id ''%s'');',
    csid
  );
end $$;

-- create an example foreign table
drop foreign table if exists people;
create foreign table people (
  id bigint,
  name text
)
  server my_clickhouse_server
  options (
    table 'people',
    rowid_column 'id',
    startup_cost '42'
  );
  
insert into people(id, name)
values (1, 'foo');

select * from people;