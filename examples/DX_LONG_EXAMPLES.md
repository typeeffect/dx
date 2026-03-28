# DX Long Examples

## Purpose

These examples are meant to test whether the language still reads well outside toy snippets.

They are not all expected to type-check today.
They are design validation fixtures.

## 1. Recursive function

```dx
fun fact(n: Int) -> Int:
    if n <= 1:
        1
    else:
        n * fact(n - 1)
    .
.
```

## 2. Local pipeline with `it`

```dx
fun active_emails(path: Str) -> List[Str] !py !throw:
    from py pandas import read_csv

    read_csv(path)
    it'filter(_'active)
    it'map(_'email)
.
```

## 3. Logging with `lazy`

```dx
fun debug(enabled: Bool, msg: () -> Str !io) -> Unit !io:
    if enabled:
        print(msg())
    else:
        ()
    .
.

fun run(path: Str) -> Unit !io:
    debug(true, lazy read_text(path))
.
```

## 4. Default and named parameters

```dx
fun connect(host: Str, *, port: Int = 5432, ssl: Bool = true) -> Conn !io:
    ...
.

fun open_default() -> Conn !io:
    connect("db.local", port: 6432, ssl: false)
.
```

## 5. Partial application

```dx
fun ids(users: List[User]) -> List[Int]:
    users'map(_'id)
.
```

## 6. Block-bodied lambda

```dx
fun net_amount(lines: List[Line]) -> List[Money]:
    lines'map:
        x =>:
            val gross = x'price * x'qty
            gross - x'discount
        .
    .
.
```

## 7. Full anonymous function

```dx
fun sorter() -> ((User, User) -> Bool):
    fun(a: User, b: User) -> Bool:
        a'age < b'age
    .
.
```

## 8. Python boundary

```dx
from py pandas import read_csv

fun load(path: Str) -> PyObj !py !throw:
    read_csv(path)
.
```

## 9. Nested member chains

```dx
fun city(user: User) -> Str:
    user'account'primary_address'city
.
```

## 10. Zero-ary block closure

```dx
fun cache_or_compute(key: Str, compute: () -> Value !io) -> Value !io:
    if cache'has(key):
        cache'get(key)
    else:
        val value = compute()
        cache'set(key, value)
        value
    .
.

fun load_user(id: Int) -> Value !io:
    cache_or_compute("user:" + id'str(), lazy:
        read_user_from_disk(id)
    .)
.
```

## 11. Variadics

```dx
fun sum(xs: Int...) -> Int:
    xs'reduce((a, b) => a + b)
.
```

## 12. Keyword-only config

```dx
fun render(title: Str, *, width: Int = 80, color: Bool = true) -> Str:
    ...
.
```

## 13. Simple ADT handling

```dx
type Result[A, E]
    = Ok(A)
    | Err(E)
.

fun unwrap_or_zero(x: Result[Int, Str]) -> Int:
    match x:
        Ok(v):
            v
        Err(_):
            0
    .
.
```

## 14. Structured-concurrency-shaped API sketch

```dx
fun fetch_all(ids: List[Int]) -> List[User] !wait !io:
    spawn_all(ids'map(id => lazy fetch_user(id)))'join_all()
.
```

## 15. Query expression sketch

```dx
fun high_value_orders(pg: Db, parquet: Table[Order]) -> Queryable[OrderSummary]:
    query:
        from u in pg'users
        join o in parquet on u'id == o'user_id
        where o'total > 100
        select { email: u'email, total: o'total }
    .
.
```

## 16. Query over in-memory objects

```dx
fun high_value_local(orders: List[Order]) -> List[Order]:
    query:
        from o in orders
        where o'total > 100
        select o
    .
.
```

## 17. Query-on-query composition

```dx
fun active_users(pg: Db) -> Queryable[User]:
    query:
        from u in pg'users
        where u'active
        select u
    .
.

fun active_emails(pg: Db) -> Queryable[Str]:
    query:
        from u in active_users(pg)
        select u'email
    .
.
```

## 18. Data transform sketch

```dx
fun normalize_users(path: Str) -> Table[UserRow] !py !throw:
    from py pandas import read_csv

    read_csv(path)
    it'rename(createdAt: "created_at")
    it'derive(full_name: _'first_name + " " + _'last_name)
    it'select(id: _'id, full_name: _'full_name, created_at: _'created_at)
.
```

## 19. Service handler sketch

```dx
fun get_report(id: Int) -> Report !io !wait !py !throw:
    val raw = fetch_report(id)
    val enriched = enrich(raw)
    debug(true, lazy "report ready: " + id'str())
    enriched
.
```

## 20. Receiver `me`

```dx
fun full_name() -> Str:
    me'first + " " + me'last
.
```

## 21. Nested conditionals

```dx
fun classify(x: Int) -> Str:
    if x < 0:
        "neg"
    elif x == 0:
        "zero"
    else:
        if x < 10:
            "small"
        else:
            "big"
        .
    .
.
```

## 22. Placeholder-heavy transformation

```dx
fun gross(lines: List[Line]) -> List[Money]:
    lines'map(_'price * _'qty)
.
```

## 23. Deferred fallback

```dx
fun get_or_else[T](opt: Option[T], fallback: () -> T) -> T:
    match opt:
        Some(x):
            x
        None:
            fallback()
    .
.

fun load_name(user: User) -> Str:
    get_or_else(user'nickname, lazy user'full_name())
.
```

## 24. Python wrapper sketch

```dx
from py numpy import array

fun to_numpy(xs: List[Int]) -> PyObj !py !throw:
    array(xs)
.
```

## 25. Long member-oriented expression

```dx
fun final_url(cfg: Config) -> Str:
    cfg'environments'prod'services'api'base_url
.
```
