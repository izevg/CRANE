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
      insert assets; insert job('analyze.package'); NOTIFY job   // задача в той же транзакции, без outbox
      insert audit_event
    tx.commit()                                          // точка видимости
    cache.invalidate(md:<repo>:<name>)                    // + pub/sub на все реплики
```

Сбой до `commit` оставляет только объект в `staging/`, который удаляется правилом lifecycle. Сбой между копированием в CAS и коммитом оставляет blob без ссылок, его заберёт GC после grace period.

## 10.2 Proxy fetch: singleflight и stale-while-revalidate

```rust
fn fetch(repo, key):                                    // key = нормализованный путь upstream
    if e = l1.get(key) or l2.get(key):                   // L1 (moka в процессе), затем L2 (Valkey)
        if e.fresh(): return e
        if e.stale_ok(): spawn(refresh(repo, key)); return e.with_header("Age")
    return singleflight(key, || {                        // один fetch на кластер
        lock = valkey.set_nx("sf:"+key, token, 30s)
        if !lock: return wait_for(key, timeout=25s)      // подписка на "inv" до SET NX, затем чтение кэша
        resp = upstream.get(key, if_none_match=e?.etag)  // SSRF guard, лимиты, таймауты
        match resp:
          304            => touch(e)
          200 metadata   => body = format.rewrite_upstream(resp); store(key, body, ttl(key))
          200 blob       => tee(resp.stream → client, → blob_store staging); verify; commit CAS
          404            => store_negative(key, neg_ttl(key))
          5xx | timeout  => if e: return e.stale_if_error() else 502 с диагностикой   // после фильтра served(), см. ниже
        publish("inv", "px:"+key)
    })
```

Размер тела ограничен до записи: документ (метаданные) собирается в память с потолком 64 МиБ, поток архива режется на пороге `CRANE_PROXY_MAX_ARTIFACT_BYTES` (по умолчанию 2 ГиБ) ещё на входе в staging, поэтому сверх лимита на диск не уходит ни байта. Превышение даёт 502 (`upstream_failure="too-large"`), blob не коммитится, запись в кэш не создаётся, и следующий запрос снова идёт в upstream. Предел действует на один поток: N одновременных запросов разных ключей могут занять в staging до N × предел.

Внутри одного процесса singleflight реализуется через общий `Shared<Future>`, поэтому сотни одновременных запросов одного архива ждут один поток байт. L1 внутри Valkey-адаптера когерентна между репликами: каждая запись в кэш и снятие lease публикуют имя ключа в канал `inv`, подписчик сбрасывает L1; ждущий подписан на `inv` (одна подписка на процесс) и регистрируется до `SET NX`, поэтому снятие lease между `SET NX` и ожиданием не теряется. Между репликами работает блокировка Valkey; при её недоступности реплики ходят в upstream независимо, это безопасно, потому что запись в CAS идемпотентна.

Ограничение v0.2.0: исходящий HTTP(S)_PROXY не поддерживается (клиент upstream игнорирует переменные прокси, иначе резолв ушёл бы за пределы проверки адреса); при поддержке прокси guard проверяет хост, а адрес — дело прокси.

### Видимость версий proxy

```rust
fn served(v, rules, now, caller) -> Result<(), Unavailable>:   // одно правило видимости для hosted и proxy
    if r = rules.deny(v.format, v.name, v.version):            // активные emergency_block (в том числе созданные из MAL-*)
        if !(artifact_path and break_glass(caller, v)): return Blocked(r)
    if v.row and v.row.state != Released: return v.row.state  // hold, ручная блокировка, удаление
    if v.repo.kind == proxy and !v.promoted
       and in_window(upstream_published_at(v), policy.quarantine_age, now):
        return Quarantined(until)
    Ok(())

fn document(repo, name):                                       // md: — после фильтра, px: — сырой upstream
    md.get_or_render(repo, name, || render(px.fetch(repo, name), served),
        ttl = min(ttl(px), до ближайшего конца окна карантина среди версий))
```

-   Это расширение существующей функции видимости `served()` (единственное правило видимости каталога), а не вторая функция.
-   **Документ.** Версии, которым `served()` отказал, убираются из строк индекса cargo и из `versions` и `time` packument. Dist-tag, указывающий на скрытую версию, переводится на наибольшую отдаваемую версию не выше версии тега и не pre-release (если тег указывал на стабильную), иначе тег убирается.
-   **Артефакт.** `served()` проверяется на пути скачивания независимо от документа: lockfile с `resolved` идёт мимо packument. Координату (name, version) CRANE вписывает в URL файлов при переписывании документа; разбор имени файла остаётся запасным путём для старых URL.
-   **stale-if-error.** Фильтр стоит после `px:`, поэтому stale-документ проходит тот же `served()`: заблокированная версия не отдаётся ни из свежего, ни из stale-кэша.
-   **Индекс правил deny** живёт в памяти реплики и перестраивается по событию в канале `inv` (`deny(format, name)`, `decision(repo, name)`), как префиксное дерево namespaces; подписчик сбрасывает `md:*:<name>` формата. Полная перезагрузка по таймеру страхует от потерянного сообщения. Срок вступления блокировки в силу на всех репликах не больше 5 с (как у отзыва, 10.4), в худшем случае — период страховочной перезагрузки.
-   **Время публикации upstream** читает адаптер формата: `upstream_published_at(doc, version) -> Option<время>`. Источники: npm `time[версия]`; PyPI `upload-time` (PEP 700); NuGet registration `published`; Cargo — необязательное поле `pubtime` строки sparse-индекса ([registry index](https://doc.rust-lang.org/cargo/reference/registry-index.html); у старых версий отсутствует); Maven — `Last-Modified` файла. `None` обрабатывает настройка политики `unknown_age: quarantine | release`, без подстановки времени первой выдачи.
-   **Commit артефакта proxy** — это хвост `publish` из 10.1 с принципалом `upstream`: строки `package_version`, `asset` и `blob` в одной транзакции, задача `attestation.verify` ставится там же (10.6.1 получает свою версию). Поэтому GC (10.9) и `crane restore verify` видят proxy-объекты.
-   Если одна purl есть в двух proxy-репозиториях, у неё две строки; finding группируется по (advisory, purl) (14.7).

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
    audit("package.claimed", principal, ns, name)        // владелец = команда namespace (или команда принципала без namespace, 14.15)
```

Индекс namespaces хранится в памяти каждой реплики как префиксное дерево и перестраивается по событию изменения. Поиск занимает O(длины имени).

## 10.4 Авторизация и применение отзыва

```rust
fn authorize(req):
    p = authenticate(req)                                 // JWT OIDC-провайдера | crp_ | crr_ | crci_
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

Токены CRANE имеют формат `<префикс><8 символов идентификатора><32 байта base62><crc32>`, префиксы: `crp_` личный, `crr_` робот, `crci_` нагрузка (раздел 13). Префикс позволяет найти запись без перебора, CRC позволяет secret-сканерам GitLab отличать настоящие токены от случайных строк. Храним HMAC-SHA256 с pepper: для высокоэнтропийного секрета это безопасно и в сотни раз быстрее argon2 на горячем пути. Команда атрибуции фиксируется при выпуске credential: 14.15.

```rust
fn issue(req, kind):                                     // POST /tokens, /robots/{id}/credentials
    p = authenticate(req)
    require(p.credential in [BffSession, IdpUserLogin])  // токен CRANE и client credentials → 403
    if kind in [personal, robot_secret]: require(context.mfa)   // свежий MFA (раздел 13)
    require(req.scopes ⊆ rights(p, kind))                // только сужение
    ttl = min(req.ttl, policy.max_ttl(kind))
    if kind == session: ttl = min(ttl, 12h, p.idp_refresh_expiry)
    // естественное истечение сессии IdP; досрочный конец — Back-Channel Logout → эпоха (10.4, 13)
    // команда атрибуции фиксируется при выпуске: 14.15
    return mint(prefix(kind), scopes = req.scopes, ttl)
```

Правила выпуска и их причины — раздел 13.

## 10.5 Обмен OIDC-токена платформы на credential

```rust
fn exchange(id_token):
    jwt = verify(id_token, jwks(gitlab_issuer))          // подпись, iss, aud = "crane", exp, nbf
    c = jwt.claims    // project_path, source_project_path, namespace_path, ref, ref_type, ref_protected,
                      // pipeline_source, job_id, pipeline_id, user_login, environment
    rules = trust_policy.matching(c.project_path)         // явные проекты, не wildcard по умолчанию
    scopes = rules.filter(|r| r.matches(c.ref, c.ref_type, c.ref_protected)).union()
    // запись только из защищённого ref вне merge request; проект-источник MR отличается от целевого у форка;
    // иначе только чтение; scope publish — только имена из правила trust policy
    can_write = c.ref_protected and c.ref_type in [tag, branch]
                and c.pipeline_source != "merge_request_event"
                and c.source_project_path == c.project_path
    if !can_write: scopes = scopes.read_only()
    require(!scopes.is_empty())
    // команда атрибуции фиксируется при выпуске: 14.15
    ttl = min(rules.max_ttl, 1h, job_timeout)              // по умолчанию 15 мин; со scope записи 5 мин
    return issue("crci_", scopes, ttl, provenance = {project, commit, pipeline, job, ref, ref_protected, pipeline_source})
```

Тот же алгоритм обслуживает любого доверенного издателя: Kubernetes (projected ServiceAccount token), GitHub Actions, сервисный аккаунт Keycloak; trust policy задаёт издателя и допустимые claims. Каждая публикация из CI получает provenance из claims токена, а не из того, что сообщил клиент. Эти claims пишутся и как evidence (`predicate_type = https://slsa.dev/provenance/v1`, `kind = jwt`, `trust = crane`, `signer_issuer` = `iss` токена, `signer_san` = `project_path` без ref (ref законно меняется от версии к версии), `facts` = `repo` (тот же `project_path`), commit, pipeline, job, `ref`, `ref_protected`, `pipeline_source`), поэтому gate provenance и правило регрессии (10.6.1) читают один источник и для hosted. Такая provenance выпущена CRANE, а не подписана сборочной платформой, поэтому сама по себе не означает SLSA Build L2; L2 дают attestations из CI (`trust = publisher`). Это закрывает требование связи публикации с project, commit, pipeline и job.


## 10.6 Promotion и gates

```rust
fn evaluate(version, target_stage):
    gates = policy.gates_for(version.repo, target_stage)  // cve, license, provenance, quality
    results = gates.map(|g| match evidence.latest(version.id, g.predicate_types):
        None                      => Missing             // «проверка отсутствует»
        Some(e) if e.invalid      => Deny                // подпись или subject не сошлись
        Some(e) if e.unverifiable => AnalysisError       // «ошибка анализа»: сбой скана, нет trust root, нет сети
        Some(e) if stale(e, g)    => Stale
        Some(e)                   => g.judge(e, exceptions.active(version)))
    verdict = combine(results)      // любой Deny → Deny; Missing/Stale → по политике gate
    if verdict.allow and approvals.satisfied(version, target_stage):
        tx: version.state = Released; insert promotion; outbox('version.promoted')
    return verdict                   // всегда с перечнем причин для UI
```

Gate не знает видов доказательств: он объявляет, какие `predicate_types` читает (раздел 9). Provenance читает SLSA Provenance, а `trust` отличает claims CRANE от подписанной provenance CI и upstream; cve читает vulns и VEX; license читает CycloneDX и SPDX; quality читает quality gate SonarQube. Требование «без валидной attestation не продвигать» — настройка gate provenance по репозиторию и стадии: какие `trust` он принимает (`publisher`, `upstream`, `crane` для claims) и требует ли `valid`. Версия без подходящего evidence не уходит дальше quarantine. Gate cve требует evidence скана артефакта самим CRANE (`trust = crane`); SBOM издателя дополняет, но не заменяет его.

Promotion меняет только строку в БД: digest и байты те же. Повторная оценка запускается событиями «новое evidence», «новая версия базы уязвимостей», «истекло исключение», «изменилась политика».

### 10.6.1 Проверка upstream attestations (proxy)

Задача `attestation.verify` ставится после commit CAS и идёт вне горячего пути: скачивание клиенту не ждёт проверки; состояние версии до результата определяет политика репозитория (quarantine новых версий из upstream, 14.2): окно карантина по возрасту применяет `served()` (10.2), hold по регрессии пишется в строку версии. Источник зависит от формата:

| Формат | Где брать | Что приходит |
| --- | --- | --- |
| npm | `dist.attestations` в packument; `GET /-/npm/v1/attestations/<name>@<version>` | SLSA provenance и publish attestation в Sigstore bundle |
| PyPI | Integrity API: `GET /integrity/<project>/<version>/<filename>/provenance` | PEP 740: in-toto Statement в DSSE, издатель Trusted Publishing |
| Maven Central | `.asc` (PGP) и `.sigstore.json` рядом с файлом, где опубликован | Подпись файла; Sigstore bundle опционален, PGP обязателен |

NuGet и Cargo в v1.1.0 не проверяются.

Общий шаг 1: `subject` digest (у подписи файла — сам подписанный файл) равен SHA-256 blob; иначе `invalid`. Остальное зависит от вида подписи.

Sigstore bundle (`kind = sigstore`: npm, PyPI, `.sigstore.json` в Maven):

2. Цепочка сертификата Fulcio и его SAN и issuer; они кладутся в `signer_*` и в `facts` (builder, repo, commit, run).
3. Inclusion proof и signed entry timestamp Rekor берутся из самого bundle, без обращения к журналу: проверка работает офлайн.
4. Trust root — `trusted_root.json` Sigstore, обновляется через TUF; версия корня пишется в `verify_detail`.

PGP (`kind = pgp`, Maven `.asc`): подпись проверяется по публичному ключу издателя, отпечаток ключа ложится в `signer_subject`, `tlog` NULL; Fulcio, Rekor и TUF не участвуют. `valid` означает, что подпись сходится с ключом, который берётся из того же места, что и файл; доверие к ключу даёт история версий, а смену ключа ловит правило регрессии.

Ошибка самой проверки (нет trust root, upstream недоступен, незнакомый формат bundle) даёт `unverifiable`, а не `invalid`: иначе сбой инфраструктуры блокировал бы чужие релизы.

```rust
on attestation_verified(new_version):                     // attestation.verify → правило регрессии
    past = previous_versions(new_version.package)
    signer(x) = match x.kind { pgp      => x.signer_subject,                       // отпечаток ключа
                               sigstore => (x.signer_issuer, x.facts.source_repo, x.facts.workflow_path),
                               _        => (x.signer_issuer, x.signer_san) }       // jwt
    for prev in evidence.latest_valid_per_type(past):      // каждый predicate_type, что был valid раньше
        e = evidence.latest(new_version.id, [prev.predicate_type])
        if e.is_none() or e.invalid or signer(e) != signer(prev) or e.facts.repo != prev.facts.repo
           or (prev.facts.ref_protected and !e.facts.ref_protected):   // из защищённого ref в незащищённый
            hold(new_version, quarantine | blocked)        // по политике репозитория
            siem(SupplyChainSignal(new_version, prev, e)); break   // сигнал захвата аккаунта maintainer'а
```

Для Sigstore сравнивается не SAN целиком: у GitHub Actions и PyPI Trusted Publishing он содержит `@refs/tags/vX.Y.Z` и меняется каждый релиз. Берутся издатель (OID 1.3.6.1.4.1.57264.1.8), репозиторий (1.15 Source Repository Identifier, при пустом — 1.12 Source Repository URI) и путь workflow (1.9 Build Signer URI без `@ref`). Ref (1.14), триггер (1.20) и окружение раннера (1.11) идут в `facts`. Источник: github.com/sigstore/fulcio, docs/oid-info.md.

Регрессия — главный практический сигнал: у прошлых версий была валидная attestation или подпись, а у новой её нет, она `invalid` или сменились издатель и репозиторий. Так может выглядеть публикация с украденным токеном мимо CI. Карточка версии показывает «собрано в <CI> из <repo>@<commit>» из `facts`.

**Attestations из своего CI (hosted).** CI прикладывает in-toto/SLSA provenance и другие attestations к версии рядом с SBOM. Их проверяет тот же верификатор; они ложатся в evidence с `trust = publisher`, а `subject` сверяется с blob при приёме. Цель — SLSA Build L2 (hosted build platform, подписанная provenance); L3 при hardened builders остаётся документированной целью, а не гарантией.

## 10.7 Уязвимости: сопоставление, воздействие и приоритет

Два независимых пути дают findings. Advisory из фидов сопоставляются с инвентарём по PURL без сканирования байтов; Trivy пересканирует SBOM при выходе новой базы. Оба пути сходятся в одной модели finding и одном расчёте приоритета (раздел 14.6).

```rust
on advisory_changed(adv):                                 // advisory.sync → advisory.match
    if adv.withdrawn: close_findings(adv, reason = Withdrawn); return
    for v in inventory.versions_by_name(adv.ecosystem, adv.name_norm):   // индекс, не полный обход
        if format(adv.ecosystem).in_ranges(v.version, adv.ranges):
            upsert_finding(adv, v, fixed = adv.nearest_fix(v.version))
on trivy_db_updated(new_db):                              // раз в несколько часов
    for sbom in evidence.by_types([CYCLONEDX, SPDX]): enqueue(scan.sbom(sbom, new_db))   // по recency
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

Proxy-объекты достижимы через `asset` строк proxy-репозитория и собираются так же; класс `proxy-cache` может иметь свой retention.

## 10.10 Поиск: гибридное ранжирование

Поиск сочетает лексическую и векторную ноги, слияние по рангам, необязательный реранкер и эвристики порядка. Основа: статьи владельца [1] и [2], документация pgvector и PostgreSQL. Числа из [1] и [2] получены на RAG-корпусе (вики и реестр, векторы размерности 2000), а не на CRANE. Своё у CRANE: фильтр ACL до ранжирования и эмбеддинги на пакет (раздел 9, `package_embedding`). В [1] ACL нет.

```
fn search(q, principal, filters):
    acl = visible_repos(principal)                          // фильтр ДО ранжирования
    for class in [hosted, proxy]:                           // свой список на класс: proxy-кэш не вытесняет внутренние пакеты [1]
        lex[class] = pg: search_tsv @@ AND-запрос; если мало — OR до 7 термов [1]; + similarity(name_norm, q) (pg_trgm)  // top 100
        vec[class] = pg: package_embedding WHERE model = '<read_model>' JOIN package ON acl
                     ORDER BY embedding::halfvec(N) <=> embed(q) LIMIT 100
                     (hnsw.iterative_scan = relaxed_order, затем пересортировка по расстоянию)
    rrf_d   = Σ_leg w_leg / (60 + rank_leg(d))              // w_vec = w_lex = 0,2 (стартовые, [1])
    if reranker.available: rerank(top 40 by rrf)            // глубину не урезать ради скорости [1]
    base_d  = reranker ? 0,6·norm(rerank_d) + rrf_d : rrf_d
    kept    = { d : base_d ≥ floor } ∪ top1                 // сначала состав: выдача не пустая
    final_d = base_d × Π boosts_11.4(d, principal)          // потом порядок; свежесть только из 11.4
    return top(kept by final_d, 20) with highlights
```

Без итеративного скана HNSW возвращает не больше `hnsw.ef_search` строк, поэтому `ef_search` не меньше `LIMIT` ноги или включённый итеративный скан.

| Параметр | Значение | Основание |
| --- | --- | --- |
| Тип вектора | `halfvec`, размерность ≤ 1024 | Индекс должен помещаться в память; HNSW по `halfvec` — до 4 000 измерений (pgvector) |
| `m`, `ef_construction` | 32, 256 (по умолчанию pgvector 16 и 64) | Качество графа; см. ниже |
| `hnsw.ef_search` | итеративный скан включён, `hnsw.ef_search` = 100 (= LIMIT ноги); без итеративного скана `ef_search` ≥ LIMIT обязательно | Иначе строк вернётся меньше `LIMIT` (по умолчанию 40) |
| `hnsw.iterative_scan` | `relaxed_order` (pgvector ≥ 0.8.0) | Поиск с фильтром ACL |
| `hnsw.max_scan_tuples` | 20 000 по умолчанию | Предел работы итеративного скана; вместе с `hnsw.scan_mem_multiplier` |
| RRF: константа и веса ног | 60; `w_vec` = `w_lex` = 0,2 | Слияние по рангам [1]; стартовые значения, калибруются на своём наборе запросов |
| Вес реранкера | 0,6 | Стартовое значение [1] |
| Глубина реранка | до 40 кандидатов | [1] |

**Векторная нога**

-   **Почему `halfvec`.** В [2] индекс `vector(2000)` с m = 16 занял 14 ГБ и не влез в 12 ГБ `shared_buffers`; `halfvec(2000)` с m = 32 и ef_construction = 256 дал 7,2 ГБ, и индекс поместился. Размер индекса относительно памяти важнее `ef_search` и CPU. Оценки размера для CRANE: раздел 9, «Масштаб L: эмбеддинги».
-   **Поиск с ACL и итеративные сканы.** В [2] Recall@40 с фильтром относительно точного поиска: 0,15–0,31 без итеративного скана (`ef_search` 50); 0,56–0,78 с `relaxed_order` на старом графе; 0,80–0,86 при ≈ 20–30 мс после пересборки с m = 32 и ef_construction = 256. Рост `ef_search` со 100 до 200 на старом графе почти ничего не дал: узким местом было качество графа. `relaxed_order` требует пересортировки результата через материализованный CTE (на PG 17+ нужен `+ 0`, README pgvector). Если ACL оставляет очень малую долю строк, точный скан по отфильтрованному набору может выиграть у индекса (README: точный поиск хорош для условий с малым процентом совпадений); решает планировщик, проверяйте `EXPLAIN`.
-   **Частичные индексы, ловушки** ([PostgreSQL docs](https://www.postgresql.org/docs/17/indexes-partial.html)).
    1. Предикат запроса должен следовать из предиката индекса, и PostgreSQL доказывает это на этапе планирования. Параметр `model = $1` в обобщённом prepared-плане индекс `WHERE model = 'm'` не использует (параметризованные условия с частичными индексами не работают). Поэтому модель берётся из конфигурации как проверенный литерал (allowlist `[a-z0-9._-]`) и тот же каст `embedding::halfvec(N)`.
    2. Предикаты пишутся одинаково: в [2] ORM выдал `IS TRUE` против индекса с `= true`, PG 17 индекс не сопоставил, и вышло 252 мс вместо 0,1 мс в 69 местах.
    3. Каждый UPDATE и DELETE несёт полный предикат: в [2] пропущенный `NOT is_deleted` вызвал ≈ 27 тыс. полных сканов за ночь (≈ 133 мс на документ).
    4. Тест через `EXPLAIN` проверяет, что поисковые запросы используют индекс.
-   **Пересборка без простоя и смена модели.** `CREATE INDEX CONCURRENTLY` под временным именем, затем `DROP INDEX CONCURRENTLY` старого и переименование. Задача идемпотентна: сверяет `pg_class.reloptions` (m, ef_construction) и `pg_index.indisvalid`, удаляет невалидные остатки неудачного CIC, пересобирает только при необходимости [2]. Шаги смены модели описаны в разделе 9; строки, оставленные «для отката», получают срок хранения [2].

**Лексическая нога**

-   По умолчанию GIN по `search_tsv` и pg_trgm по `name_norm`. RUM (отдельное расширение, в обязательный набор не входит) — вариант для L, где важно ранжирование внутри индекса [2].
-   Цена мёртвых строк [2]: после перечанкинга p50 лексики вырос со 175 до 252 мс из-за ≈ 178 тыс. мёртвых строк, оставленных для отката; после удаления, VACUUM и `REINDEX CONCURRENTLY` p50 191 мс (p95 653 мс), RUM 806 → 368 МБ, HNSW 8,4 → 7,2 ГБ, ≈ 11 мин без блокировки записи. Для CRANE: UPDATE `package` (описание, lifecycle) создают мёртвые кортежи, поэтому нужна настройка autovacuum и нельзя держать устаревшие строки.

**Слияние и порядок** [1]

-   Формула одна, она в псевдокоде выше. RRF суммирует вклады рангов ног с весами, а не сырые оценки; реранкер добавляет нормированную оценку с весом 0,6.
-   «Сначала состав, потом порядок»: состав выдачи решает порог `floor` (первый результат остаётся всегда, выдача не пуста), и только затем применяются множители 11.4 (lifecycle deprecated/EOL, популярность, свежесть). В [1] своя формула свежести; в CRANE свежесть берётся только из 11.4.
-   **Реранкер** [1]: cross-encoder по ≤ 40 кандидатам. Qwen3-Reranker-4B FP8 против bge-reranker-v2-m3: nDCG@10 0,789 → 0,813, key-doc R@10 0,722 → 0,755, p50 0,63 с; сокращение глубины до 25×1 стоило ≈ 4,6 п.п. nDCG@10. CRANE использует настраиваемый rerank-endpoint (раздел 13), модель выбирает оператор; без реранкера работают порядок слияния и порог.
-   **Абляция** [1]: без векторной ноги key-doc R@10 падает на 13,0 п.п., без лексической на 2,2 п.п. (R@50 на 5,2 п.п.); обе ноги остаются.

**Измерение** [2]

-   Recall@k считается относительно точного поиска с боевыми настройками (итеративный скан включён): в [2] инструмент с устаревшими настройками показал 0,43 вместо 0,77.
-   Пересборка индекса с теми же параметрами в [2] изменила top-10 для 69 из 111 эталонных запросов. При сравнении версий пайплайна индекс фиксируют или повторяют сборки и измеряют разброс.

**Источники**

1.  «Гибридный поиск для RAG: как «ноги» усиливают друг друга», 2026-10-25, <https://ezhukov.space/posts/hybrid-search-rag.md>.
2.  «Индексы под гибридный поиск в PostgreSQL: halfvec, HNSW и RUM на практике», 2026-10-25, <https://ezhukov.space/posts/postgres-hybrid-indexes.md>.
3.  pgvector README: <https://github.com/pgvector/pgvector> (итеративные сканы с 0.8.0, `ef_search` 40 по умолчанию, HNSW по `halfvec` до 4 000 измерений); документация PostgreSQL о частичных индексах: <https://www.postgresql.org/docs/17/indexes-partial.html>.

## 10.11 Outbox и webhooks

Outbox служит только внешней доставке (webhooks, SIEM; решается в P2 / v1.1.0); внутренние задачи идут строкой `job` в транзакции данных (раздел 9). Событие записывается в `outbox` в той же транзакции. Relay выбирает пачки через `FOR UPDATE SKIP LOCKED`, создаёт доставки по подпискам и помечает событие. Доставка подписывается заголовком `Crane-Signature: t=<unix>,v1=HMAC-SHA256(secret, t + "." + body)`, несёт `Crane-Event-Id` для дедупликации и повторяется с экспоненциальной задержкой и jitter (10 с … 1 ч) в течение 24 часов, после чего попадает в DLQ с возможностью ручного повтора из UI.

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
                                      // requirements с hash, packages.lock.json, CycloneDX/SPDX;
                                      // из lockfile берутся только координаты и хэши, `resolved` и URL игнорируются
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

## 10.16 Членство, атрибуция и суточный агрегат

Роли людей из IdP попадают в CRANE одним путём: нормализатор claims (13) даёт множество групп, `group_mapping` превращает его в права, `observe` приводит открытые строки `role_binding` источника в соответствие с правами. Cedar читает только `role_binding` (14.15).

```rust
// observe: вызывается при входе, обмене, обновлении crp_, опросе каталога, admin event и SCIM
fn observe(p, source, groups: GroupClaim):
    let CompleteGroups(g) = match groups {
        Complete(g) => g,
        Overage | Absent => return Unchanged,            // неизвестно не равно пусто: ничего не закрываем
    }
    wanted = resolve(g, mappings)                        // чистая функция домена, ниже
    open   = role_binding.open_rows(p, source)
    tx:
        for r in open - wanted:  close(r, valid_to = now()); audit("access.role_revoked")
        for r in wanted - open:  insert(r, valid_from = now()); audit("access.role_granted")
        if changed: p.epoch += 1; valkey.publish("inv", Epoch(p.id))   // отзыв ≤ 5 с (10.4)

fn resolve(g: &CompleteGroups, mappings: &[GroupMapping]) -> BTreeSet<Grant>:
    g.filter(|ref| mappings.has(ref))                    // группа без маппинга ничего не даёт
     .flat_map(|ref| mappings.for(ref))                  // (workspace_id, role) или роль экземпляра при workspace_id = NULL
     .filter(|m| m.workspace is None or m.workspace.active)   // архивной команде новые строки не открываются
     .collect()
```

`valid_from` и `valid_to` ставит момент наблюдения, а не изменения в IdP; открытая строка одна на факт (`role_binding_open`), поэтому повторное наблюдение идемпотентно. Локальные строки (`source = local`) `observe` не трогает: их меняет owner команды через API.

```rust
// attribute: чистая функция, команда скачиваний фиксируется при выпуске credential
fn attribute(principal, requested: Option<WorkspaceId>, teams_now: &[WorkspaceId])
    -> Result<Attribution, AttributionError>:
    match principal.kind:
        robot | workload => Team(principal.workspace)    // requested игнорируется
        user => match requested:
            Some(w) if w in teams_now => Team(w)
            Some(_)                   => Err(NotAMember)
            None if teams_now.len() == 1 => Team(teams_now[0])
            None                      => Unattributed    // «не распределено»
```

Результат записывается в `credential.workspace_id` и больше не меняется; смена состава команд посреди сессии атрибуцию токена не меняет до перевыпуска. Скачивание копирует значение в `download_event.workspace_id` без вычислений.

```rust
// context_mfa: чистая функция, вход для Cedar-атрибута context.mfa
fn context_mfa(auth: &AuthStrength, cfg: &MfaConfig, now) -> bool:
    strong = auth.acr in cfg.acr_accepted
          or auth.amr intersects cfg.amr_accepted
          or cfg.assume_idp_enforced
    strong and auth.auth_time is Some(t) and now - t <= cfg.max_age   // нет auth_time: не свежо, fail closed
```

Задачи воркера:

```rust
// usage.rollup_day: после дайджеста последнего часа суток UTC; идемпотентна
fn usage.rollup_day(d: Date):
    require(digest_written(hour = d 23:00 UTC))
    tx:
        DELETE FROM download_daily WHERE day = d
        INSERT INTO download_daily
          SELECT d, e.workspace_id, e.repository_id, e.purl, e.source, p.kind, count(*), sum(e.bytes)
          FROM download_event_<d> e JOIN principal p ON p.id = e.actor
          GROUP BY e.workspace_id, e.repository_id, e.purl, e.source, p.kind

// access.directory_sync: раз в 5 мин, если Directory настроен
fn access.directory_sync():
    members = {}                                         // subject -> группы, найденные в каталоге
    for g in group_mapping.distinct_groups():            // единицы-десятки групп
        for m in directory.members(g):
            if m.active: members[m.subject].insert(g)
            else:        principal(m.subject).epoch += 1 // отключён в IdP: отзыв ≤ 5 с по 10.4
    for p in principals_with_open_rows(idp) + members.keys():
        observe(p, idp, Complete(members[p.subject] or {}))   // кого в группе больше нет, у того строки idp закрываются
```

Число уникальных людей и машин агрегат не хранит. Опрос каталога и приём push (admin events, SCIM) вызывают один и тот же `observe`: 13.
