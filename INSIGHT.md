# INSIGHT.md — Limites do Rust, Adaptações Idiomáticas e API Pura do Zod v4 (`rust-zod`)

Baseado na inspeção direta do código-fonte do **Zod v4** (`colinhacks/zod`, `packages/zod/src/v4/`), este documento define **o que entra**, **o que foi depreciado no Zod v4 (e não traremos do v3)**, **os limites da linguagem Rust sem macros** e **os padrões idiomáticos (Closure Builders)** que usaremos.

---

## 1. Limpeza Radical: Apenas API Nativa do Zod v4 (Zero Legado do Zod v3)

Ao inspecionar `packages/zod/src/v4/classic/schemas.ts`, fica claro que o Zod v4 **depreciou dezenas de métodos encadeados do v3** e moveu formatos para funções top-level (`z::*`). Como estamos construindo o `rust-zod` do zero sobre o conceito do Zod v4, **não vamos carregar métodos depreciados do v3**:

### 1.1. Formatos de String são Top-Level (`z::email()`, `z::uuid()`, etc.)
No Zod v3 usava-se `z.string().email()` e `z.string().uuid()`. **No Zod v4 isso está `@deprecated`!**
No `rust-zod`, todos os formatos de string são construtores de primeiro nível em `z::`:
- **Identificadores**: `z::uuid()`, `z::uuidv4()`, `z::uuidv6()`, `z::uuidv7()`, `z::guid()`, `z::nanoid()`, `z::cuid2()`, `z::ulid()`, `z::xid()`, `z::ksuid()`
- **Rede & Web**: `z::email()`, `z::url()`, `z::http_url()`, `z::hostname()`, `z::ipv4()`, `z::ipv6()`, `z::mac()`, `z::cidrv4()`, `z::cidrv6()`
- **Codificação & Tokens**: `z::base64()`, `z::base64url()`, `z::hex()`, `z::jwt()`, `z::emoji()`, `z::e164()`, `z::credit_card()`, `z::iban()`, `z::currency_code()`, `z::hash(...)`, `z::string_format(...)`
- **Datas/Horas ISO (`z::iso::*`)**: `z::iso::date()`, `z::iso::time()`, `z::iso::datetime()`, `z::iso::duration()`

E o que fica em `z::string()`? **Apenas checks e transforms genéricos de texto**:
- Checks: `.min(n)`, `.max(n)`, `.length(n)`, `.nonempty()`, `.regex(re)`, `.includes(s)`, `.starts_with(s)`, `.ends_with(s)`, `.lowercase()`, `.uppercase()`
- Transforms: `.trim()`, `.normalize()`, `.to_lower_case()`, `.to_upper_case()`, `.slugify()`
*(Obs.: Todos os tipos de formato como `z::email()` ou `z::uuid()` também aceitam `.min()`, `.max()`, `.trim()`, `.meta()`, `.optional()`, etc.)*

### 1.2. Tipos Numéricos são Top-Level (`z::int()`, `z::float64()`, etc.)
No Zod v4:
- `z.number().int()`, `.safe()`, `.finite()` e `.step()` foram depreciados/substituídos.
- Números já rejeitam `Infinity` e `NaN` por padrão.
- Os tipos numéricos são construtores top-level diretos:
  - `z::number()` / `z::float64()`, `z::float32()`
  - `z::int()`, `z::int32()`, `z::uint32()`, `z::int64()`, `z::uint64()`, `z::bigint()`
  - Checks em números: `.gt(v)`, `.gte(v)` / `.min(v)`, `.lt(v)`, `.lte(v)` / `.max(v)`, `.positive()`, `.nonnegative()`, `.negative()`, `.nonpositive()`, `.multiple_of(v)`.

### 1.3. Objetos no Zod v4 (`z::object`, `z::strict_object`, `z::loose_object`)
No Zod v4:
- `.passthrough()` foi depreciado em favor de `.loose()` e do construtor `z::loose_object(...)`.
- `.merge()` foi depreciado em favor de `.extend(...)`.
- Temos os 3 construtores diretos:
  - `z::object(|o| ...)` (faz `strip` de chaves desconhecidas por padrão)
  - `z::strict_object(|o| ...)` (rejeita chaves desconhecidas com `unrecognized_keys`)
  - `z::loose_object(|o| ...)` (preserva chaves desconhecidas)

### 1.4. `z::options(...)` em vez de `z::enum_()`
Como `enum` é palavra reservada no Rust (`enum_` ou `r#enum` são feios) e ter múltiplos nomes (`enum_`, `enum_builder`, `enum_map`) fragmenta a API, usamos **`z::options(...)`** — nome que já é usado pelo próprio Zod v4 tanto na propriedade `.options` quanto na mensagem de erro oficial (`"Invalid option: expected one of ..."`).
Assim como o `z.enum([...])` do Zod, **`z::options(...)` recebe sempre e somente uma lista de opções** (array, slice ou `Vec`, via trait `IntoOptions`) — sem metadados por opção e sem formas alternativas.

---

## 2. Objetos Literais (`{ ... }`) vs. Closure Builders (`|b| ...`) no Rust

No TypeScript, o Zod v4 recebe objetos literais anônimos para metadados e configurações:
```ts
// TypeScript Zod v4
z.email().meta({ id: "Email", title: "User Email", description: "Primary email" })
```

### O limite no Rust:
Rust **não possui objetos literais anônimos** nem parâmetros opcionais nomeados.

### A Solução Idiomática no `rust-zod`: Closure Builders (`|...|`)

#### 2.1. `.meta(|m| ...)` (Sem `.describe()`)
No Zod v4, `.describe(str)` é apenas um atalho legado para `globalRegistry.add(cl, { description })`.
No `rust-zod`, **cortamos `.describe()`** e concentramos tudo no Closure Builder `.meta(|m| ...)`:
```rust
z::email().meta(|m| {
    m.id("UserEmail")
     .title("E-mail do Usuário")
     .description("Endereço principal de contato")
     .example("alice@example.com")
     .deprecated(false)
     .custom("x-pii", true) // extensões OpenAPI / JSON Schema
})
```

#### 2.2. Objetos sem Macros e sem Decorators: `z::object(|o| ...)`
```rust
let user_schema = z::object(|o| {
    o.field("id", z::uuid())
     .field("email", z::email())
     .field("name", z::string().min(2).trim())
     .field("age", z::int32().min(18).optional())
     .field("role", z::options(["admin", "member", "guest"]))
})
.meta(|m| m.id("User").description("Entidade de usuário"));
```

#### 2.3. Customização Pontual de Mensagem de Erro
- Por padrão, nenhum método exige mensagem (`z::email()`, `z::string().min(3)`) — o catálogo de locale ativo (`pt-BR`, `en`, `es`) gera a mensagem padrão automaticamente.
- Para sobrescrever a mensagem de um schema ou check específico, oferecemos o sufixo `_err` / `_with` ou encadeamento:
```rust
z::email().error("E-mail inválido")
z::string().min_with(8, |e| e.error("A senha deve ter pelo menos 8 caracteres").abort(true))
```

#### 2.4. Polimorfismo via Traits (`IntoOptions`, `IntoLiterals`, `IntoFieldMask`)
Como Rust não tem function overloading tradicional, usamos traits de conversão para manter **um único nome de método** onde o TS precisava de overloads ou máscaras `{ key: true }`:
- **`z::literal(...)`**: aceita tanto um valor único `z::literal("active")` ou `z::literal(42)` quanto múltiplos valores `z::literal(["active", "pending"])` (novidade do Zod v4).
- **`object.pick(...)` / `object.omit(...)`**: no TS era obrigatório passar `{ id: true, email: true }` por causa do type-system do TS. Em Rust, passamos diretamente um array/slice: `user_schema.pick(["id", "email"])` e `user_schema.omit(["password"])`.
- **`object.partial(...)` / `object.required(...)`**: `user_schema.partial()` torna todos os campos opcionais, e `user_schema.partial_keys(["bio", "age"])` torna apenas as chaves listadas opcionais.

---

## 3. A Arquitetura `Schema<T>`: Tipagem Forte Sem Explosão da `target/`

Se codificássemos cada *regra* no sistema de tipos (`Min<Max<Email<StringSchema>>>`), o compilador Rust monomorfizaria milhares de combinações genéricas, explodindo a pasta `target/`. Por outro lado, se `Schema` não tivesse tipo algum, `.parse()` só retornaria `Value` dinâmico e métodos de número apareceriam em schemas de string.

### A Solução: `Schema<T = Value>` com AST Erased (`Arc<SchemaDef>`)
```rust
pub struct Schema<T = Value> {
    def: Arc<SchemaDef>,
    _marker: PhantomData<fn() -> T>,
}
```
1. **Apenas o Tipo de Saída `T` no Genérico**:
   - `z::string()`, `z::email()`, `z::uuid()` → `Schema<String>` (e encadear `.min(3).max(20).trim()` **continua sendo `Schema<String>`**, sem aninhar tipos!).
   - `z::int32()` → `Schema<i32>`, `z::int64()` → `Schema<i64>`, `z::float64()` → `Schema<f64>`.
   - `z::boolean()` → `Schema<bool>`.
   - `z::options(["admin", "user"])` → `Schema<String>`.
2. **Métodos Restritos por Tipo em Tempo de Compilação**:
   - Métodos de texto (`.min()`, `.trim()`, `.starts_with()`, `.regex()`) são implementados em `impl Schema<String>`.
   - Métodos numéricos (`.gt()`, `.lte()`, `.positive()`, `.multiple_of()`) são implementados em `impl<N: Numeric> Schema<N>` (onde `z::int32().min(10)` exige `10: i32`!).
   - Métodos de array (`.min()`, `.nonempty()`, `.unique()`, `.unwrap()`) são implementados em `impl<T> Schema<Vec<T>>`.
3. **Composição Automática de Tipos (`Vec<T>`, `Option<T>`, Structs/Enums)**:
   - `z::array(schema: Schema<T>)` retorna automaticamente **`Schema<Vec<T>>`**!
   - `schema.optional()` / `.nullable()` em `Schema<T>` retorna automaticamente **`Schema<Option<T>>`**!
   - Em `z::object(|o| ...)` (que nasce `Schema<Value>`), os campos aceitam qualquer `Schema<F>` heterogêneo (pois internamente guarda apenas o `Arc<SchemaDef>`), e você pode tanto chamar `.parse_into::<User>(&val)` quanto fixar o tipo no próprio schema com `.into_type::<User>()` (virando **`Schema<User>`**):
     ```rust
     let user_schema: Schema<User> = z::object(|o| {
         o.field("id", z::uuid())
          .field("age", z::int32().min(18))
     }).into_type::<User>();

     // Agora .parse() já retorna User direto:
     let user: User = user_schema.parse(&val)?;

     // E ao compor com z::array ou .optional(), o tipo vai junto automaticamente!
     let users: Vec<User> = z::array(user_schema.clone()).parse(&val)?;
     let maybe_user: Option<User> = user_schema.optional().parse(&val)?;
     ```
4. **Zero Explosão na `target/`**:
   - Toda a validação da árvore, resolução de i18n e exportação para JSON Schema / OpenAPI / GraphQL rodam em funções **100% não-genéricas** sobre `&SchemaDef`.
   - O parâmetro `T` (`PhantomData`) tem tamanho zero em memória e só atua na fronteira final para converter o `Value` validado em `T`.
### 3.1. `Arc<SchemaDef>` com Copy-on-Write (`Arc::make_mut`)
Em Rust, passar um schema para múltiplos objetos ou compartilhar entre threads de um servidor HTTP/GraphQL causaria erros de *ownership* (`use of moved value`) se o `Schema` não for barato de clonar e `Send + Sync`:
- `Schema` encapsula um `Arc<SchemaDef>`.
- Durante o encadeamento fluente (`z::string().min(3).max(20).trim()`), o `refcount` do `Arc` é `1`, então `Arc::make_mut` modifica a struct **in-place sem realocar**.
- Quando um schema base é reutilizado (`let id = z::uuid();`), `id.clone()` custa apenas um incremento atômico ($O(1)$), e qualquer customização posterior (`.optional()`) faz *Copy-on-Write* com segurança.

### 3.2. `Result` Nativo do Rust Elimina `.safe_parse()`, `.safe_decode()`, `.safe_encode()` e `.spa()`
No JavaScript, `.parse()` lança exceção (`throw new ZodError()`) e `.safeParse()` existe apenas porque `try/catch` em JS não é tipado e é lento.
Em Rust **não existem exceções**:
- `.parse(&val)` já retorna `Result<Value, ZodError>`!
- Quem quer o comportamento de `.parse()` usa `?` (ou `.unwrap()`).
- Quem quer o comportamento de `.safeParse()` faz `match schema.parse(&val)` ou `if let Ok(data) = schema.parse(&val)`.
- **Resultado**: Cortamos 6 métodos redundantes (`safe_parse`, `safe_parse_async`, `spa`, `safe_decode`, `safe_encode`, `safe_decode_async`, `safe_encode_async`) e a struct artificial `SafeParseResult`, usando `Result` idiomático do Rust em tudo!

### 3.3. Schemas Recursivos (`z::lazy`) Sem Stack Overflow (`OnceLock`)
No JS, `const Category = z.lazy(() => CategoryObj)` referencia a mesma instância em memória.
Em Rust, `fn category() -> Schema { z::object(|o| o.field("sub", z::array(z::lazy(category)))) }` chamaria `category()` recursivamente criando um novo `Arc` a cada passo e estourando a pilha ao exportar para JSON Schema ou GraphQL.
- **Solução**: O nó `Lazy` armazena um `OnceLock<Schema>` interno (avalia a função uma única vez na primeira leitura) e os exportadores rastreiam o ponteiro do próprio nó `Lazy` (além do `meta.id`), resolvendo ciclos em `#/$defs/...` sem risco de loop infinito.

### 3.4. Async Runtime-Agnostic (Zero `tokio` no `Cargo.toml`)
Para não inflar a pasta `target/` com `tokio` ou `futures`:
- `.parse_async()`, `.decode_async()` e `.encode_async()` usam apenas `core::future::Future` e `Pin<Box<dyn Future<Output = ...> + Send + Sync>>` nativos do Rust (`core`/`alloc`).
- Funciona em qualquer runtime (`tokio`, `smol`, `async-std`, WASM) com **zero dependências externas de async**.
- Se o usuário chamar `.parse()` (síncrono) em um schema que contém `refine_async` / `transform_async` / `codec_async`, retornamos erro limpo no `Result` (equivalente ao `$ZodAsyncError` do Zod v4) em vez de dar `panic!`.

### 3.5. Type-Safety Real ("Parse, don't validate") e Ponte Natural para quem vem do TypeScript
Em bibliotecas Rust baseadas em decorators (`validator`, `garde`), a validação ocorre **após** o `serde` já ter instanciado a struct inválida na memória (e se o dev esquecer de chamar `.validate()?`, dados inválidos vazam para o domínio; se um tipo falhar no JSON, o `serde` aborta no primeiro erro sem rodar validações nem i18n).
No `rust-zod`:
- **Quem vem do TypeScript se sente em casa no primeiro minuto**: escreve `z::object(|o| ...)`, `z::email()`, `z::string().min(3).trim()`, `.optional()`, `.pick(["id", "email"])`, `.partial()` exatamente como no Zod v4, com **100% de autocomplete nativo do `rust-analyzer`** (sem programar dentro de strings de macros `#[validate(...)]`).
- **Fim da multiplicação de structs**: em vez de criar 5 structs com `#[derive]` repetidos (`User`, `CreateUser`, `UpdateUser`, `PublicUser`), você deriva schemas em uma linha com `.omit(...)`, `.pick(...)`, `.partial()` e `.extend(...)`.
- **`.parse_into::<T>()` Universal em Qualquer `Schema` (Objetos, Arrays, Options, Tuplas, Primitivos)**:
  Em vez de existir apenas em `z::object`, `.parse_into::<T>(&val)` (junto com `.parse_into_async::<T>()` e `.decode_into::<T>()`) vive na base de **todo `Schema`**:
  ```rust
  // Em objetos:
  let user: User = user_schema.parse_into(&val)?;
  // Em arrays (de primitivos ou de structs):
  let users: Vec<User> = z::array(user_schema).parse_into(&val)?;
  let tags: Vec<String> = z::array(z::string().min(2)).parse_into(&val)?;
  // Em options (enums):
  let role: Role = z::options(["admin", "user"]).parse_into(&val)?;
  // Em tuplas e opcionais:
  let coords: (f64, f64) = z::tuple([z::float64(), z::float64()]).parse_into(&val)?;
  let nick: Option<String> = z::string().optional().parse_into(&val)?;
  ```
  Como toda a validação pesada roda em código não-genérico sobre `Value` e a conversão para `T` acontece apenas uma vez na saída após o schema aprovar os dados, a pasta `target/` continua minúscula e você tem tipagem forte em qualquer nível da árvore.

---

## 4. Enums Sem Macros e Sem Palavra Reservada: `z::options(...)`

Para evitar `z::enum_()` / `z::r#enum()`, usamos `z::options(...)`, que — exatamente como o `z.enum([...])` do Zod — **recebe sempre uma lista de opções** (a trait `IntoOptions` apenas aceita array, slice ou `Vec`):

```rust
let role = z::options(["admin", "user", "guest"]);
let staff = role.extract(["admin", "user"]);
let non_admin = role.exclude(["admin"]);
let opts = role.options(); // ["admin", "user", "guest"]

let status = z::options(["ACTIVE", "LEGACY"]).meta(|m| m.id("AccountStatus"));
```

- **Sem metadados por opção**: descrições/`deprecated` ficam no schema inteiro via `.meta(|m| ...)`, como no Zod.
- **Sem closure builder**: uma closure passada para um genérico `T: IntoOptions` não tem o tipo do parâmetro inferido (erro `E0282`), então a forma única de lista também evita esse problema.
- **`enum` Rust nativo**: em vez de mapeamento chave-valor no schema, a conversão para o tipo Rust fica a cargo do `serde` (ex.: `parse_into::<Role>()`), já que a saída da AST é `Value`.

---

## 5. Codecs Bidirecionais: `z::codec(...)` (`decode` / `encode`)

No Zod v4 (`z.codec(input, output, { decode, encode })`), um codec é um pipe **reversível**: `decode` vai do formato de transporte (ex.: JSON) para o formato de domínio, e `encode` faz o caminho inverso, validando os dois lados.

### 5.1. API no `rust-zod`
`decode` e `encode` são **obrigatórios**, então são parâmetros posicionais (o compilador garante os dois; um closure builder só detectaria a falta em runtime). Como os parâmetros têm tipo concreto (`Fn(Value, &mut Ctx) -> Value`), o tipo das closures é inferido normalmente:
```rust
let epoch_to_datetime = z::codec(
    z::int64(),                       // input  (transporte)
    z::iso::datetime(),               // output (domínio)
    |v, ctx| /* decode: i64 -> String ISO */,
    |v, ctx| /* encode: String ISO -> i64 */,
);

epoch_to_datetime.decode(&val)?;       // input -> output (== .parse())
epoch_to_datetime.encode(&val)?;       // output -> input
epoch_to_datetime.decode_async(&val).await?; // + encode_async
```
- Falhas dentro de `decode`/`encode` são reportadas via `ctx.issue(...)` (gera `Custom`), como no `ctx.issues.push(...)` do Zod.
- Versão assíncrona: `z::codec_async(...)` com closures que retornam `Future`, executadas por `decode_async`/`encode_async`.

### 5.2. Regras de direção (iguais ao Zod)
- `.decode()` é equivalente a `.parse()`; `.encode()` percorre a árvore **ao contrário**: em `a.pipe(b)`, primeiro `b` e depois `a`.
- Checks/refinements rodam nas duas direções.
- `.transform(...)` é unidirecional: encontrá-lo durante `.encode()` gera erro de encode (não é uma `ZodIssue`), em vez de `panic!`.
- O codec se propaga por objetos, arrays, unions etc.: `z::object(|o| o.field("created_at", epoch_to_datetime.clone()))` também suporta `.encode()`.

### 5.3. Codecs prontos
- `z::stringbool()`: `"true"/"1"/"yes"/"on"/"y"/"enabled"` ↔ `bool` (customizável com `.truthy([...])`, `.falsy([...])`, `.case_sensitive(true)`).
- Receitas da documentação do Zod (`string ↔ number`, `string ↔ int`, `json string ↔ Value`, `base64 ↔ bytes`, `epoch ↔ datetime`) podem virar helpers em `z::codecs::*`.

### 5.4. Integração com exportadores
No `to_json_schema`, `io: Input` usa o schema de entrada do codec e `io: Output` usa o de saída, como no Zod.

### 5.5. Suporte no `Value` para Tipos de Domínio do Zod
Para suportar `z::date()`, tokens codificados e codecs bidirecionais (`string ↔ Date`, `base64 ↔ Bytes`), o `Value` inclui representações nativas de:
- `Date(i64)`: timestamp Unix em milissegundos UTC.
- `Bytes(Vec<u8>)`: bytes binários brutos.

---

## 6. O Que Foi Cortado do Zod (JS-ismos e Depreciações)

| Item no Zod TS | Status no `rust-zod` | Motivo |
| :--- | :--- | :--- |
| `.describe("...")` | **Cortado** | Substituído por `.meta(\|m\| m.description("..."))`. |
| `.safeParse()`, `.spa()`, `.safeDecode()`, `.safeEncode()` | **Cortado** | Em Rust não há `throw`: `.parse()`, `.decode()` e `.encode()` já retornam `Result<Value, ZodError>`, servindo tanto para `?` quanto para `match Ok/Err`. |
| `z.string().email()`, `.uuid()`, `.url()`, etc. | **Cortado (Deprecated no v4)** | No Zod v4 são top-level: `z::email()`, `z::uuid()`, `z::url()`, `z::iso::datetime()`, etc. |
| `z.number().int()`, `.safe()`, `.finite()`, `.step()` | **Cortado (Deprecated no v4)** | No Zod v4 são top-level: `z::int()`, `z::int32()`, `z::uint32()`, `z::float64()` e `.multiple_of()`. |
| `.passthrough()`, `.merge()`, `.removeDefault()`, `.removeCatch()` | **Cortado (Deprecated no v4)** | No Zod v4 usam-se `.loose()` / `z::loose_object()`, `.extend()`, e `.unwrap()`. |
| `z.enum([...])` | **Renomeado para `z::options([...])`** | Evita a palavra reservada `enum` no Rust (`enum_`). Recebe sempre uma lista de opções, como no Zod. |
| `z.nativeEnum()` / `z.enum(EnumObj)` | **Cortado** | Enums Rust nativos são convertidos via `serde` (`parse_into::<T>()`), não pelo schema. |
| `schema.pick({ a: true })` / `omit({ b: true })` | **Simplificado para `["a", "b"]`** | No TS usa-se `{ a: true }` por limitação do `keyof` no type-system. Em Rust, `schema.pick(["id", "email"])` e `schema.omit(["password"])` são muito mais limpos. |
| `pipe.in` / `pipe.out` | **Renomeado para `.input()` / `.output()`** | `in` é palavra reservada no Rust (`for x in y`). `.input()` e `.output()` evitam `r#in`. |
| `z.instanceof(Class)` | **Cortado** | Específico de herança de protótipos JS (`instanceof`). Não se aplica a schemas de dados em Rust / JSON / GraphQL. |
| `z.undefined()`, `z.void()`, `z.nan()`, `z.symbol()`, `z.promise()`, `z.function()` | **Cortado** | Artefatos do runtime JS sem sentido em Rust, JSON Schema, OpenAPI ou GraphQL. |

---

## 7. Taxonomia de Erros, Formatadores e Locales do Zod v4

Extraído diretamente de `v4/core/errors.ts` e `v4/locales/`:

### 7.1. As 11 Issues do Zod v4 (`ZodIssueCode`)
1. `InvalidType { expected }`
2. `TooBig { origin, maximum, inclusive, exact }`
3. `TooSmall { origin, minimum, inclusive, exact }`
4. `InvalidFormat { format, pattern, prefix, suffix, includes }`
5. `NotMultipleOf { divisor }`
6. `UnrecognizedKeys { keys }`
7. `InvalidUnion { errors, discriminator, options, inclusive, matches }` (cobre `union`, `discriminated_union` e `xor`)
8. `InvalidKey { origin, issues }`
9. `InvalidElement { origin, key, issues }`
10. `InvalidValue { values }` (unifica `literal` e `enum`)
11. `Custom { params }`

### 7.2. Os 4 Formatadores do Zod v4
- `error.flatten()` → `{ form_errors, field_errors }`
- `error.format()` → árvore com `_errors`
- `error.treeify()` → `{ errors, properties, items }` (novo no Zod v4)
- `error.prettify()` → string formatada com `✖ ... \n  → at ...` (novo no Zod v4)

### 7.3. i18n e Troca de Locales (`en`, `pt-BR`, `es`, etc.)
- Baseado em `v4/locales/en.ts`, `pt.ts`, `es.ts` (com suporte a unidades dimensionáveis `caracteres`/`elementos`/`bytes` e gênero/artigos).
- Troca global (`z::set_locale(Locale::PtBr)`) ou por execução (`schema.parse_with(&val, |opt| opt.locale(Locale::Es))`).

---

## 8. Otimizações no Rust: Performance de Baixo Nível Transparentes ao DX (Zero Heap no Happy Path)

Otimizações de performance no motor são guiadas pelo princípio de **zero overhead no happy path**, garantindo que o compilador Rust gere código extremamente veloz **sem nunca poluir a DX com tempos de vida (`'a`)** e mantendo **conversão automática** para os tipos padrão da linguagem:

### 8.1. Pilha Interna de Path com Auto-Conversão (`SmallVec` → `Vec<PathSegment>`)
- **Regra**: Durante a descida na árvore de validação (`users[0].address.street`), o `ParseContext` utiliza internamente uma pilha em memória contígua na stack (ex: `SmallVec<[PathSegment; 8]>`).
- **Zero Heap no Happy Path**: Em requisições válidas (a esmagadora maioria em produção), **nenhuma alocação no heap** é realizada para rastrear paths.
- **Spillover transparente**: Caso a profundidade do objeto aninhado exceda a capacidade inline da stack (ex: > 8 níveis), o `SmallVec` derrama automaticamente para o heap sem risco de panic ou estouro de pilha.
- **Auto-conversão na emissão da issue**: Ao instanciar `ZodIssue`, converte via `.into_vec()` para o `Vec<PathSegment>` padrão, mantendo a API pública desacoplada da crate `smallvec`.

### 8.2. Otimização de Strings sem Lifetimes (SSO Transparente na DX)
- **Regra de ouro**: **Zero contaminação por lifetimes (`'a`)**. O usuário nunca deve ser obrigado a lidar com `Schema<'a>` ou `Value<'a>`, o que inviabilizaria schemas estáticos globais e compartilhamento limpo entre threads.
- **Transparência total**: A otimização de strings utiliza internamente Small String Optimization (SSO — via `compact_str` ou buffer inline de até 24 bytes), onde nomes de chaves de objetos, identificadores e status residem diretamente na stack dentro do enum `Value`.
- Para o consumidor da biblioteca, a DX é 100% nativa: aceita `&str` e `String`, expõe `.as_str() -> &str` e implementa `Display` sem qualquer atrito.

### 8.3. Copy-on-Write com `Arc::make_mut` e Zero-Allocation Method Chaining
- `Schema` opera internamente com `Arc<SchemaDef>`.
- Durante a construção fluente de schemas (`z::string().min(3).max(20).trim()`), o `refcount` do `Arc` é 1, logo o Rust modifica a estrutura **in-place na memória sem realocar** novos nós.
- Quando compartilhado entre threads em servidores HTTP ou GraphQL, `clone()` custa apenas um incremento atômico de 64 bits ($O(1)$).

### 8.4. Fast-Path de Desserialização Direta (Bypass de Serde para Casos Identitários)
- Para `Schema<Value>`, o método `.parse(&val)` realiza bypass instantâneo de serializadores intermediários, retornando diretamente a referência ou clone de `Value`.
- Para tipos primitivos conhecidos (`f64`, `i64`, `bool`, `String`), a extração ocorre diretamente dos variants do enum `Value` sem serialização intermediária para JSON.

### 8.5. Lazy Recursivo com Memoização `OnceLock` Zero-Recompute
- `z::lazy(|| ...)` avalia a closure do schema apenas uma única vez de forma thread-safe (`OnceLock`) e reaproveita o ponteiro interno na AST, eliminando re-execução redundante e prevenindo loops infinitos em exportadores e validações recursivas.

---

## 9. Exportadores (JSON Schema, OpenAPI, GraphQL) e Importador (`from_json_schema`)

1. **JSON Schema & OpenAPI (`to_json_schema`)**:
   - Baseado em `v4/core/to-json-schema.ts`.
   - Targets: `Draft202012` (default), `Draft07`, `Draft04`, `OpenApi30`, `OpenApi31`.
   - Suporte a `io: Input | Output`, detecção de ciclos/reuso via `$defs` usando `.meta(|m| m.id("..."))`, e callback `override_with(...)` (evitando a palavra reservada `override`).
2. **Importação Reversa (`z::from_json_schema`)**:
   - Baseado em `v4/classic/from-json-schema.ts`.
   - Como nossa AST existe em runtime (ao contrário de crates baseadas em proc-macros), conseguimos construir um `Schema` dinâmico do `rust-zod` a partir de um documento JSON Schema externo (`draft-2020-12`, `draft-07`, `openapi-3.0`).
3. **GraphQL (`to_graphql_sdl` / Objetos GraphQL)**:
   - Gera `type` (`Object`), `input` (`InputObject`), `enum` (a partir de `z::options([...])`, com description do `.meta()` do schema), `union`, `scalar`.
   - Inversão automática de nullability (`!` por padrão nos campos obrigatórios; sem `!` quando `.optional()` ou `.nullable()`).
   - Nomeação via `.meta(|m| m.id("User"))` ou inferência automática em PascalCase para sub-objetos aninhados (`UserAddress`).

