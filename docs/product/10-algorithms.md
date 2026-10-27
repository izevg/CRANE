# 10. Алгоритмы

Ниже описаны алгоритмы, от которых зависят корректность и производительность. Псевдокод близок к будущей реализации на Rust.

## 10.1 Атомарная публикация

Гарантии: незавершённая загрузка невидима; release нельзя перезаписать; две параллельные публикации одной версии дают ровно один успех; повтор с тем же `Idempotency-Key` возвращает тот же результат.

```rust
fn publish(principal, repo, upload):
    plan  = format.parse_upload(upload)                 // имя, версия, файлы, metadata
    decide(principal, Publish, plan.package)?            // Cedar + namespace ownership
    check_limits(plan)?                                  // размер, число файлов, квота
    store = storage.class_of(repo)                        // primary | proxy-cache | archive | restricted
    for file in plan.files:                              // потоково, без буферизации
        digests = blob_store.put_staging(upload_id, file.stream)   // sha256/512/1, md5 за проход
        verify_client_checksum(file, digests)?           // если клиент прислал хэш
    tx = db.begin()
      pg_advisory_xact_lock(hash(repo, plan.package))    // сериализация по пакету
      if exists(version) and !mutable: return Conflict(409, "release immutable")
      for f: store.commit(upload_id, f.sha256)            // идемпотентный copy в CAS этого хранилища
             insert blob ON CONFLICT DO NOTHING
      insert package ON CONFLICT DO NOTHING; insert package_version(state = initial_state(policy))
      insert assets; insert outbox('version.published'); enqueue(analyze.package)
      insert audit_event
    tx.commit()                                          // точка видимости
    cache.invalidate(md:{repo}:{name})                    // + pub/sub на все реплики
```

Сбой до `commit` оставляет только объект в `staging/`, который удаляется правилом lifecycle. Сбой между копированием в CAS и коммитом оставляет blob без ссылок, его заберёт GC после grace period.

## 10.2 Proxy fetch: singleflight и stale-while-revalidate

```rust
fn fetch(repo, key):                                    // key = нормализованный путь upstream
    if e = l1.get(key) or l2.get(key):                   // moka в процессе, затем Valkey
        if e.fresh(): return e
        if e.stale_ok(): spawn(refresh(repo, key)); return e.with_header("Age")
    return singleflight(key, || {                        // один fetch на кластер
        lock = valkey.set_nx("sf:"+key, token, 30s)
        if !lock: return wait_for(key, timeout=25s)      // pub/sub "ready:"+key, затем чтение кэша
        resp = upstream.get(key, if_none_match=e?.etag)  // SSRF guard, лимиты, таймауты
        match resp:
          304            => touch(e)
          200 metadata   => body = format.rewrite_upstream(resp); store(key, body, ttl(key))
          200 blob       => tee(resp.stream → client, → blob_store staging); verify; commit CAS
          404            => store_negative(key, neg_ttl(key))
          5xx | timeout  => if e: return e.stale_if_error() else 502 с диагностикой
        publish("ready:"+key)
    })
```

Внутри одного процесса singleflight реализуется через общий `Shared<Future>`, поэтому сотни одновременных запросов одного архива ждут один поток байт. Между репликами работает блокировка Valkey; при её недоступности реплики ходят в upstream независимо, это безопасно, потому что запись в CAS идемпотентна.

## 10.3 Защита внутренних имён

```rust
fn resolve(format, name, client_repo):
    ns = namespace_index.longest_match(format, name)     // trie по префиксам и scope
    if ns.reserved:
        if client_repo.kind == proxy: return NotFound(reason = "reserved_internal_name")
        return hosted_lookup(ns.repos, name)             // только hosted, никогда upstream
    return client_repo.kind == hosted ? hosted_lookup(...) : proxy.fetch(...)

fn claim_name(principal, format, name):                  // первая публикация имени
    ns = namespace_index.longest_match(format, name)
    require(ns != None or policy.allow_unscoped(format)) // иначе 403 с подсказкой
    require(decide(principal, CreatePackage, ns))
    record namespace_owner_history
```

Индекс namespaces хранится в памяти каждой реплики как префиксное дерево и перестраивается по событию изменения. Поиск занимает O(длины имени).

## 10.4 Авторизация и применение отзыва

```rust
fn authorize(req):
    p = authenticate(req)                                 // JWT Keycloak | crp_ | crr_ | crci_
    key = (p.id, p.epoch, action, resource.scope_key)
    if d = decision_cache.get(key): return d              // moka, TTL 30 с
    d = cedar.is_authorized(p.entity, action, resource.entity, context)
    d.trace = d.diagnostics.reason_policy_ids            // какое правило сработало
    decision_cache.put(key, d); return d

on revoke(credential) | block(user) | groups_changed(user):
    db: revoked_at = now(); epoch += 1
    valkey.publish("inv", Epoch(p.id))                   // все реплики сбрасывают кэш ≤ 5 с
    // если pub/sub потерян, TTL 30 с ограничивает окно
```

Токены CRANE имеют формат `<префикс><8 символов идентификатора><32 байта base62><crc32>`, префиксы: `crp_` личный, `crr_` робот, `crci_` нагрузка (раздел 13). Префикс позволяет найти запись без перебора, CRC позволяет secret-сканерам GitLab отличать настоящие токены от случайных строк. Храним HMAC-SHA256 с pepper: для высокоэнтропийного секрета это безопасно и в сотни раз быстрее argon2 на горячем пути.

## 10.5 Обмен OIDC-токена платформы на credential

```rust
fn exchange(id_token):
    jwt = verify(id_token, jwks(gitlab_issuer))          // подпись, iss, aud = "crane", exp, nbf
    c = jwt.claims    // project_path, namespace_path, ref, ref_type, ref_protected,
                      // pipeline_source, job_id, pipeline_id, user_login, environment
    if c.pipeline_source == "merge_request_event" and from_fork(c): scopes = read_only
    rules = trust_policy.matching(c.project_path)         // явные проекты, не wildcard по умолчанию
    scopes = rules.filter(|r| r.matches(c.ref, c.ref_type, c.ref_protected)).union()
    require(!scopes.is_empty())
    ttl = min(rules.max_ttl, 1h, job_timeout)              // по умолчанию 15 мин
    return issue("crci_", scopes, ttl, provenance = {project, commit, pipeline, job})
```

Тот же алгоритм обслуживает любого доверенного издателя: Kubernetes (projected ServiceAccount token), GitHub Actions, сервисный аккаунт Keycloak; trust policy задаёт издателя и допустимые claims. Каждая публикация из CI получает provenance из claims токена, а не из того, что сообщил клиент. Это закрывает требование связи публикации с project, commit, pipeline и job.

## 10.6 Promotion и gates

```rust
fn evaluate(version, target_stage):
    gates = policy.gates_for(version.repo, target_stage)  // cve, license, provenance, quality
    results = gates.map(|g| match evidence.latest(version.digest, g.kind):
        None                      => Missing             // «проверка отсутствует»
        Some(e) if e.failed       => AnalysisError       // «ошибка анализа»
        Some(e) if stale(e, g)    => Stale
        Some(e)                   => g.judge(e, exceptions.active(version)))
    verdict = combine(results)      // любой Deny → Deny; Missing/Stale → по политике gate
    if verdict.allow and approvals.satisfied(version, target_stage):
        tx: version.state = Released; insert promotion; outbox('version.promoted')
    return verdict                   // всегда с перечнем причин для UI
```

Promotion меняет только строку в БД: digest и байты те же. Повторная оценка запускается событиями «новое evidence», «новая версия базы уязвимостей», «истекло исключение», «изменилась политика».

## 10.7 Уязвимости: сопоставление, воздействие и приоритет

Два независимых пути дают findings. Advisory из фидов сопоставляются с инвентарём по PURL без сканирования байтов; Trivy пересканирует SBOM при выходе новой базы. Оба пути сходятся в одной модели finding и одном расчёте приоритета (раздел 14.6).

```rust
on advisory_changed(adv):                                 // advisory.sync → advisory.match
    if adv.withdrawn: close_findings(adv, reason = Withdrawn); return
    for v in inventory.versions_by_name(adv.ecosystem, adv.name_norm):   // индекс, не полный обход
        if format(adv.ecosystem).in_ranges(v.version, adv.ranges):
            upsert_finding(adv, v, fixed = adv.nearest_fix(v.version))
on trivy_db_updated(new_db):                              // раз в несколько часов
    for sbom in evidence.sboms_by_recency(): enqueue(scan.sbom(sbom, new_db))
on finding_new_or_changed(f):
    impact = graph.upstream_closure(f.purl)               // рекурсивный CTE ниже
    scores = scorer.score_all(f, impact).ok()             // systemone; None → только правила
    f.priority = max(rules::priority(f, impact), thresholds::priority(scores))
    apply_policy(f)                                       // notify | issue | block_promotion | emergency
    outbox(security.finding.*); siem(VulnerabilityFinding(f, scores))
```

```sql
WITH RECURSIVE up(version_id, depth) AS (
  SELECT e.version_id, 1 FROM dependency_edge e WHERE e.dep_purl = $1
  UNION
  SELECT e.version_id, up.depth + 1 FROM dependency_edge e
  JOIN package_version v ON e.dep_purl = v.purl JOIN up ON v.id = up.version_id
  WHERE up.depth < 12)
SELECT ... JOIN usage_evidence u ON ...  -- downloaded < lockfile < sbom < deployed
```

Уровни достоверности не смешиваются: в UI видно «скачивали», «в lockfile», «в SBOM сборки», «развёрнуто» отдельно. Отсутствие находок отображается как «известных уязвимостей не найдено на дату и версию базы», а не «безопасно».

## 10.8 Лицензии: SPDX expressions и оценка вирусности

```rust
fn determine(version):                                   // источник с наибольшим приоритетом
    candidates = [manual_review, sbom_declared, metadata_declared, license_file_detected(conf ≥ 0.9)]
    if conflicting(candidates): flag(Conflict)            // показываем все источники
    return first_present(candidates) or Unknown

fn judge(expr, ctx):                                     // ctx: internal | distributed | saas | embedded
    match expr:
      Id(l)          => policy.decision(category(l), l, ctx)   // allow | review | deny
      Id(l) WITH e   => policy.decision_with_exception(l, e, ctx)
      A AND B        => worst(judge(A), judge(B))
      A OR B         => best(judge(A), judge(B))          // выбор лицензиата
      Unknown | LicenseRef-* => Review                   // никогда не Allow
    order: Allow < Review < Deny

fn judge_with_score(expr, ctx, usage):                   // usage: способ связывания, распространение
    base = judge(expr, ctx)
    v = scorer.score(LicenseVirality, features(expr, texts, ctx, usage)).ok()
    if base == Allow and v.value >= policy.virality_review: return Review   // только ужесточение
    return base                                          // Deny никогда не смягчается оценкой
```

Оценка `license_virality` от systemone отвечает на вопрос, насколько сильно обязательства лицензии распространяются на производное ПО именно в этом контексте: статическое или динамическое связывание, отдельный процесс, доступ по сети, распространение или внутреннее использование. Особенно полезна она для нестандартных текстов `LicenseRef-*` и необычных исключений, где таблица категорий молчит. Оценка поднимает решение до review и задаёт порядок очереди юристов; юридического заключения она не выносит.

Категории (permissive, weak copyleft, strong copyleft, network copyleft, proprietary/source-available, unknown/custom) и решения по контекстам задаёт организация. Встроенного правила «GPL запрещено» нет. Определение лицензии по тексту файла выполняется в песочнице библиотекой askalono (Apache-2.0) по шаблонам SPDX.

## 10.9 Сборка мусора и retention

```
retention(dry_run):                                     // правила: возраст, число версий, SNAPSHOT
    victims = rules.select(versions) - holds - released_if_protected
    report(victims, bytes_freed); if dry_run: return
    soft_delete(victims)                                  // tombstone + audit

gc.mark():                                               // лидер по advisory lock, отдельно по каждому blob_store
    UPDATE blob SET gc_mark = now()
    WHERE gc_mark IS NULL AND NOT EXISTS (asset ссылается) AND NOT EXISTS (активная загрузка)
gc.sweep():
    for b in blob WHERE gc_mark < now() - grace(24 ч):
        if still_unreferenced(b) FOR UPDATE: blob_store.delete(b); delete row
    UPDATE blob SET gc_mark = NULL WHERE снова есть ссылка   // воскрешение
```

## 10.10 Поиск: гибридное ранжирование

```rust
fn search(q, principal, filters):
    acl = visible_repos(principal)                        // фильтр ДО поиска
    lex = pg: ts_rank_cd(search_tsv, websearch_to_tsquery(q)) + similarity(name, q)   // top 200
    sem = pg: embedding <=> embed(q)  (HNSW, ef_search 64)                           // top 200
    fused = rrf(lex, sem, k = 60)                         // reciprocal rank fusion
    scored = fused.map(|d| d.score * boosts(d, principal))  // эвристики 11.4
    if reranker.available and q.is_natural_language: rerank(top 50)
    return top(scored, 20) with highlights
```

## 10.11 Outbox и webhooks

Событие записывается в `outbox` в той же транзакции. Relay выбирает пачки через `FOR UPDATE SKIP LOCKED`, создаёт доставки по подпискам и помечает событие. Доставка подписывается заголовком `Crane-Signature: t=<unix>,v1=HMAC-SHA256(secret, t + "." + body)`, несёт `Crane-Event-Id` для дедупликации и повторяется с экспоненциальной задержкой и jitter (10 с … 1 ч) в течение 24 часов, после чего попадает в DLQ с возможностью ручного повтора из UI.

## 10.12 Rate limiting (GCRA)

```lua
-- Valkey Lua, атомарно; tat = theoretical arrival time
local tat = math.max(tonumber(redis.call('GET', KEYS[1]) or 0), now)
local new_tat = tat + interval
if new_tat - now > burst * interval then return {0, new_tat - now - burst*interval} end
redis.call('SET', KEYS[1], new_tat, 'PX', burst*interval) return {1, 0}
```

Лимиты раздельные для чтения, публикации и управления. При недоступности Valkey действует локальный token bucket на реплику с лимитом, делённым на число реплик.

## 10.13 Аудит с защитой от изменения

Каждая запись содержит `hash = SHA-256(prev_hash ‖ canonical_json(event))`. Запись в таблицу идёт через роль без прав UPDATE и DELETE. Раз в час последний hash фиксируется в отдельный объект S3 с Object Lock (WORM) и отправляется в SIEM. Проверка целостности пересчитывает цепочку от последнего якоря.

## 10.14 Прогрев кэша и готовность к офлайну

```rust
fn readiness(lockfile_or_sbom):
    coords = parse(lockfile)          // package-lock, pnpm-lock, yarn.lock, poetry.lock, uv.lock,
                                      // requirements с hash, packages.lock.json, CycloneDX/SPDX
    for c in coords: status[c] = (metadata_cached?, blob_cached?, integrity_match?)
    report = {ready: %, missing: [...], integrity_mismatch: [...]}
    if warm: enqueue(cache.warm(missing))
```

## 10.15 Миграция из Nexus

1.  **Инвентаризация**: обход REST API Nexus, список hosted-репозиториев, компонентов, assets и checksums.
2.  **Перенос**: потоковое копирование каждого asset, пересчёт SHA-256, сверка с checksum Nexus, запись в CAS; версии получают state `released` и источник `import:nexus`.
3.  **Metadata**: переносится необходимая metadata формата; история скачиваний не переносится как история использования.
4.  **Сверка**: отчёт расхождений; повторный запуск идемпотентен.
5.  **Переключение**: CI templates и конфигурации клиентов меняют URL; proxy-кэш прогревается заново.
