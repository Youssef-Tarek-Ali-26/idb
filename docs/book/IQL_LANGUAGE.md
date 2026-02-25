# iQL Query Language

The iQL developer experience — how it actually feels to use.

## The Core Principle

SQL was designed for relational tables. iQL is designed for **entities in space**.
Every query reads like "find things that are like this, near that, connected to those."

## Basic Queries

```python
# Find products — reads like English
Product where price < 3000

# Multiple conditions — no AND/OR boilerplate needed, newlines imply AND
Product where
  price < 3000
  stock > 0
  category == "rings"

# Semantic search — meaning() is a first-class operator
Product where meaning("elegant traditional jewelry")

# Combine structured + semantic — this is the magic
Product where
  price < 3000
  meaning("elegant traditional")

# Sorting and limiting
Product where price < 3000 | sort(price desc) | top(10)

# Aggregates
Product where category == "rings" | count()
Product where stock > 0 | avg(price)
Product | group(category) | count()
```

## Graph Traversal

```python
# Follow relationships with ->
Brand("Norn Gold") -> Product
# "Give me all products belonging to brand Norn Gold"

# Chain hops
Company("Norn") -> Brand -> Product
# "All products from all brands owned by company Norn"

# Filter at any point in the chain
Company("Norn") -> Brand -> Product where price < 3000
# "Cheap products from any Norn brand"

# Reverse traversal with <-
Product("18K Gold Signet Ring") <- Order <- Customer
# "Which customers ordered this ring?"

# Multi-hop with filters at each level
Customer where region == "Cairo"
  -> Order where date > "2026-01-01"
  -> Product where meaning("luxury")
# "Luxury products ordered this year by Cairo customers"
```

## Semantic Search (The Differentiator)

```python
# meaning() on text — "find products whose text embedding is near this"
Product where meaning("something bold for a young woman")

# meaning() with threshold
Product where meaning("minimalist gold", threshold=0.8)

# meaning() combined with structured filters
Product where
  price in 1000..5000
  category == "necklaces"
  meaning("modern minimalist")

# Similar to a specific record
Product where similar(Product(42))
# "Find products similar to product #42 in embedding space"

# Image similarity (if image_embedding exists)
Product where image_similar(upload("reference.jpg"))
```

## Mutations

```python
# Create
create Product {
  name = "18K Gold Signet Ring"
  description = "Classic men's signet ring in 18 karat gold"
  price = 4500
  stock = 12
  category = "rings"
  image = file("signet.jpg")
  belongs_to = Brand("Norn Gold")
  supplied_by = Supplier("Cairo Goldworks")
}

# Update by ID
update Product("ring-001") { price = 4200 }

# Update by query
update Product where stock == 0 { flags = "discontinued" }

# Delete
delete Product("ring-001")
```

## Reactive Queries (Real-Time)

```python
# watch = "subscribe to this query, push diffs"
watch Product where stock < 10
# Client gets: initial results, then real-time updates as stock changes

watch Product where meaning("trending summer fashion") | top(20)
# Live-updating "trending products" feed

watch Order where customer == Customer("youssef") | sort(date desc) | top(5)
# My recent orders, live

# Reactive aggregates
watch Product where category == "rings" | avg(price)
# Live average ring price — updates on every price change
```

## Time Travel

```python
# Point-in-time query
Product("ring-001") @ "2025-06-01"
# "What did this product look like on June 1st?"

# Full history
Product("ring-001") @ history
# Returns all versions with timestamps
```

## Variables and Composition

```python
# Assign intermediate results
luxury = Product where price > 5000
cairo_customers = Customer where region == "Cairo"

# Use them
cairo_customers -> Order -> $luxury
# "Which Cairo customers bought luxury products?"

# Parameterized queries (for app code)
def low_stock(threshold: int):
  Product where stock < $threshold

low_stock(5)  # products with stock < 5
```

## Pipe Operators

```python
# Pipes chain transformations (like Unix pipes)
Product | count()                           # count all
Product | avg(price)                        # average price
Product | group(category) | count()         # count per category
Product | sort(price desc) | top(10)        # top 10 most expensive
Product | sort(created_at desc) | take(20)  # 20 most recent
```

---

## TypeScript SDK (What App Developers See)

```typescript
import { NornDB } from '@norndb/client'

const db = new NornDB({ url: 'wss://norn.example.com', token: '...' })

// Typed queries — schema generates TypeScript types
const rings = await db.query<Product>(
  `Product where category == "rings" and price < 3000`
)
// rings: Product[] with full type safety

// Reactive subscription
const unsub = db.watch<Product>(
  `Product where stock < 10`,
  {
    onInitial: (products) => renderTable(products),
    onAdd: (product) => addRow(product),
    onRemove: (product) => removeRow(product),
    onChange: (product, diff) => updateRow(product, diff),
  }
)

// Graph traversal with type inference
const products = await db.query<Product>(
  `Brand("Norn Gold") -> Product where meaning("luxury")`
)

// Semantic search
const results = await db.query<Product>(
  `Product where meaning($query) and price < $maxPrice`,
  { query: "elegant traditional", maxPrice: 3000 }
)
```

---

## Schema Definition

```python
define entity Product {
  name: string
  description: string
  price: f32
  stock: u16
  category: enum(rings, necklaces, bracelets, earrings)
  created_at: timestamp

  # Relationships
  belongs_to: -> Brand
  supplied_by: -> Supplier

  # Auto-generated embeddings
  text_embedding: embed(name, description)    # auto-embeds on write
  image_embedding: embed_image(image)         # auto-embeds uploaded images

  # Spatial config
  hilbert_dims: [price, stock, category, text_embedding]

  # Access control
  scoped by tenant

  # Indexes
  index price
  index category
}

define entity Brand {
  name: string
  country: string
  founded: u16
  owns: -> Product[]    # reverse of Product.belongs_to
}
```

---

## iQL vs SQL Comparison

| SQL | iQL |
|-----|-----|
| `SELECT * FROM products WHERE price < 3000` | `Product where price < 3000` |
| `SELECT * FROM products p JOIN brands b ON p.brand_id = b.id WHERE b.name = 'Norn'` | `Brand("Norn") -> Product` |
| Semantic search: not possible without extensions | `Product where meaning("elegant")` |
| Reactive: not possible, poll manually | `watch Product where stock < 10` |
| 3-hop join: triple JOIN nightmare | `Company -> Brand -> Product` |

The whole point: **no SELECT, no FROM, no JOIN, no GROUP BY keywords**.
Entities, arrows, pipes, and `meaning()`. A fashion merchandiser who's never
written SQL could read `Product where price < 3000 and meaning("trendy summer")`
and understand exactly what it does.

## Grammar Summary

```
query       = entity_expr [where_clause] [pipe_chain]
            | "watch" query
            | "create" entity_type "{" field_list "}"
            | "update" entity_expr "{" field_list "}"
            | "delete" entity_expr

entity_expr = IDENT                          # all of entity type
            | IDENT "(" STRING ")"           # by ID
            | entity_expr "->" IDENT         # forward traversal
            | entity_expr "<-" IDENT         # reverse traversal
            | entity_expr "@" time_expr      # time travel

where_clause = "where" predicate_list

predicate   = IDENT op value                 # structured: price < 3000
            | "meaning" "(" STRING ")"       # semantic search
            | "similar" "(" entity_expr ")"  # similarity to record
            | "image_similar" "(" expr ")"   # image similarity

pipe_chain  = "|" pipe_op [pipe_chain]

pipe_op     = "count" "()"
            | "avg" "(" IDENT ")"
            | "sum" "(" IDENT ")"
            | "min" "(" IDENT ")"
            | "max" "(" IDENT ")"
            | "group" "(" IDENT ")"
            | "sort" "(" IDENT ["asc"|"desc"] ")"
            | "top" "(" NUMBER ["," IDENT ["asc"|"desc"]] ")"
            | "take" "(" NUMBER ")"

op          = "==" | "!=" | "<" | ">" | "<=" | ">="
            | "in" range | "contains"
```
