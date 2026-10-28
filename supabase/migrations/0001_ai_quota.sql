-- Shared AI quota tracking for Pubgen (Groq translation + Cloudflare AI covers).
--
-- Apply on a fresh Supabase project: SQL Editor → paste whole file → Run
-- (or `supabase db push` when using the CLI). Safe to re-run: drops and recreates.
--
-- Model:
--   groq       — each user brings their own key; the client surfaces Groq's own
--                `x-ratelimit-remaining-requests` header locally and mirrors it here
--                keyed by a non-secret fingerprint of the key.
--   cloudflare — each account has its own 10,000 free neurons/day (reset 00:00 UTC).
--                Every successful cover reports its exact cost (4 tiles × 4.80 +
--                8 steps × 9.60 = 96.0 neurons) via `ai_cf_consume`, keyed by the
--                Cloudflare account id (not secret — it rides along on every API call).
--
-- One row per (provider, credential, UTC day). Daily reset is implicit: the RPCs
-- compare the stored `period` with the incoming one and start fresh on a new day.
--
-- The client calls these RPCs with the project's anon key only; the table itself is
-- RLS-locked (no policies = no direct reads/writes from clients).

drop table if exists ai_quota;
drop function if exists ai_groq_report(int);           -- pre-cred shape
drop function if exists ai_groq_report(text, int);
drop function if exists ai_cf_consume(numeric);        -- pre-cred shape
drop function if exists ai_cf_consume(text, numeric);
drop function if exists ai_quota_get();

create table ai_quota (
  provider text not null check (provider in ('groq', 'cloudflare')),
  cred text not null check (cred <> ''),
  period date not null default date_trunc('day', now())::date,
  groq_remaining_requests int,
  used_neurons numeric not null default 0,
  updated_at timestamptz not null default now(),
  primary key (provider, cred, period)
);

alter table ai_quota enable row level security;  -- writes only via RPC

-- Mirror of Groq's per-key remaining-requests header. `p_cred` is a short client-side
-- fingerprint of the key — never the key itself.
create or replace function ai_groq_report(p_cred text, p_requests int)
returns void language sql security definer set search_path = public as $func$
  insert into ai_quota as q (provider, cred, groq_remaining_requests)
  values ('groq', p_cred, p_requests)
  on conflict (provider, cred, period) do update set
    groq_remaining_requests =
      case when q.period < excluded.period then excluded.groq_remaining_requests
           else least(q.groq_remaining_requests, excluded.groq_remaining_requests) end,
    period = excluded.period,
    updated_at = now()
$func$;

-- Called after every successful cover generation. Returns remaining neurons today
-- for that Cloudflare account.
create or replace function ai_cf_consume(p_cred text, p_neurons numeric)
returns numeric language sql security definer set search_path = public as $func$
  with upsert as (
    insert into ai_quota as q (provider, cred, used_neurons)
    values ('cloudflare', p_cred, p_neurons)
    on conflict (provider, cred, period) do update set
      used_neurons = case when q.period < excluded.period then excluded.used_neurons
                          else q.used_neurons + excluded.used_neurons end,
      period = excluded.period,
      updated_at = now()
    returning used_neurons
  )
  select greatest(10000 - used_neurons, 0) from upsert
$func$;

-- Debug / dashboard view: one row per credential seen today. The app UI derives the
-- same numbers locally (Groq header / `ai_cf_consume` return), so this is optional.
create or replace function ai_quota_get()
returns table (provider text, cred text, remaining_requests int, remaining_covers numeric)
language sql security definer set search_path = public as $func$
  select provider, cred,
         coalesce(groq_remaining_requests, 1000),
         null::numeric
  from ai_quota
  where provider = 'groq' and period = date_trunc('day', now())::date
  union all
  select provider, cred,
         null::int,
         floor(greatest(10000 - used_neurons, 0) / 96.0)
  from ai_quota
  where provider = 'cloudflare' and period = date_trunc('day', now())::date
$func$;

-- Post-apply smoke test (run by hand, then clean up):
--   select ai_cf_consume('acctA', 96.0);   -- 9904
--   select ai_cf_consume('acctB', 96.0);   -- 9904  (separate account, separate pool)
--   select ai_groq_report('fp_aaa', 997);
--   select * from ai_quota_get();          -- 3 rows
--   delete from ai_quota;
