
--
-- PostgreSQL database dump
--

-- Dumped from database version 15.1
-- Dumped by pg_dump version 15.0

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: pool; Type: SCHEMA; Schema: -; Owner: -
--

CREATE SCHEMA pool;


--
-- Name: pay_solution(integer); Type: PROCEDURE; Schema: pool; Owner: -
--

CREATE PROCEDURE pool.pay_solution(IN solution_id_arg integer)
    LANGUAGE plpython3u
    AS $_$solution = plpy.execute(f"SELECT * FROM solution WHERE id = {solution_id_arg}", 1)
if solution.nrows() == 0:
  plpy.error("Solution id does not exist")
if solution[0]["paid"]:
  plpy.error("Solution already paid")
solution_id = solution[0]["id"]
shares = plpy.execute(f"SELECT * FROM share WHERE solution_id = {solution_id}")
if shares.nrows() == 0:
  plpy.fatal("No share data for solution")
data = {}
for share in shares:
  data[share["address"]] = share["share"]
raw_reward = solution[0]["reward"]
reward = int(raw_reward * 0.995)
total_shares = sum(data.values())
reward_per_share = reward // total_shares
def get_plan(name, stmt, types):
  if name in SD:
    return SD[name]
  plan = plpy.prepare(stmt, types)
  SD[name] = plan
  return plan
payout_plan = get_plan("payout_plan", "INSERT INTO payout (solution_id, address, amount) VALUES ($1, $2, $3)", ["integer", "text", "bigint"])
balance_plan = get_plan("balance_plan", "INSERT INTO balance (address, unpaid) VALUES ($1, $2) ON CONFLICT (address) DO UPDATE SET unpaid = balance.unpaid + $2", ["text", "bigint"])
stats_plan = get_plan("stats_plan", "INSERT INTO stats (key, value) VALUES ($1, $2) ON CONFLICT (key) DO UPDATE SET value = stats.value + $2", ["text", "bigint"])
solution_plan = get_plan("block_plan", "UPDATE solution SET paid = true WHERE id = $1", ["integer"])
try:
  with plpy.subtransaction():
    paid = 0
    for miner, share in data.items():
      amount = reward_per_share * share
      payout_plan.execute([solution_id, miner, amount])
      balance_plan.execute([miner, amount])
      solution_plan.execute([solution_id])
      paid += amount
    stats_plan.execute(["total_paid", paid])
    stats_plan.execute(["total_fee", raw_reward - reward])
    stats_plan.execute(["total_rounding", reward - paid])
	
except plpy.SPIError as e:
  plpy.fatal(f"Error while updating database: {e.args}")
$_$;


SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: balance; Type: TABLE; Schema: pool; Owner: -
--

CREATE TABLE pool.balance (
    id integer NOT NULL,
    address text NOT NULL,
    unpaid bigint DEFAULT 0 NOT NULL,
    paid bigint DEFAULT 0 NOT NULL,
    pending bigint DEFAULT 0 NOT NULL
);


--
-- Name: balance_id_seq; Type: SEQUENCE; Schema: pool; Owner: -
--

CREATE SEQUENCE pool.balance_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: balance_id_seq; Type: SEQUENCE OWNED BY; Schema: pool; Owner: -
--
