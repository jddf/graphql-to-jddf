# graphql-to-jddf

`graphql-to-jddf` generates a JDDF schema from a GraphQL schema. Some use-cases
include:

- Generating types from GraphQL (by using `graphql-to-jddf` in conjuction with
  [`jddf-codegen`][jddf-codegen]),
- Generating fake data from GraphQL (by using `graphql-to-jddf` in conjuction
  with [`jddf-fuzz`][jddf-fuzz]),
- Validating GraphQL data outside of a GraphQL server (by using
  `graphql-to-jddf` in conjunction with any JDDF implementation).

[jddf-codegen]: https://github.com/jddf/jddf-codegen
[jddf-fuzz]: https://github.com/jddf/jddf-fuzz

For example, `graphql-to-jddf` can convert the GraphQL schema:

```graphql
schema {
  query: Query
}

type Query {
  foo: String!
}
```

Into the JDDF schema:

```json
{
  "definitions": {
    "Query": {
      "properties": {
        "foo": {
          "type": "string"
        }
      }
    }
  },
  "ref": "Query"
}
```

## Usage

See `graphql-to-jddf --help` for more details, but essentially you run it like
this, using GitHub's GraphQL API as an example:

```bash
graphql-to-jddf \
  --http-endpoint=https://api.github.com/graphql \
  --http-bearer-token=YOUR_BEARER_TOKEN_HERE
```

`graphql-to-jddf` works off of _introspected GraphQL schemas_, not `.graphql`
schema files. If you're not serving your GraphQL over HTTP/HTTPS, or if you're
not using a Bearer-based auth strategy, you can also simply pass the output of
the standard introspection GraphQL query as input to `graphql-to-jddf`.

You can find the standard introspection GraphQL query in
[`src/graphql/introspection_query.graphql`][introspection-query] of this repo.

[introspection-query]: ./src/graphql/introspection_query.graphql

For example, if you pasted `introspection_query.graphql` into
[`https://graphql.org/swapi-graphql/`, the standard demo implementation of a
GraphQL server][swapi], and then copied the result to some file
`star_wars.json`, you could then run:

[swapi]: https://graphql.org/swapi-graphql/

```bash
cat star_wars.json | graphql-to-jddf
```

To generate a JDDF schema. Of course, you could have also just run:

```bash
graphql-to-jddf --http-endpoint=https://swapi-graphql.netlify.com/.netlify/functions/index
```

To get the same result.

## Examples and Caveats

Here are some examples of the sorts of JDDF schemas produced by
`graphql-to-jddf`. The GraphQL `.graphql` schemas shown in this section were
hosted using the [`example_graphql_server`][example-graphql-server] in this
repo.

[example-graphql-servier]: ./example_graphql_server/index.js

## A more complex GraphQL schema

The GraphQL schema:

```graphql
schema {
  query: Query
}

type Query {
  scalars: Scalars!
  listOfScalars: [Scalars!]!
  deepListOfScalars: [[[Scalars!]!]!]!
}

type Scalars {
  a: String!
  b: ID!
  c: Boolean!
  d: Int!
  e: Float!
}
```

Becomes:

```json
{
  "definitions": {
    "Query": {
      "properties": {
        "listOfScalars": {
          "elements": {
            "ref": "Scalars"
          }
        },
        "deepListOfScalars": {
          "elements": {
            "elements": {
              "elements": {
                "ref": "Scalars"
              }
            }
          }
        },
        "scalars": {
          "ref": "Scalars"
        }
      }
    },
    "Scalars": {
      "properties": {
        "d": {
          "type": "int32"
        },
        "b": {
          "type": "string"
        },
        "e": {
          "type": "float64"
        },
        "a": {
          "type": "string"
        },
        "c": {
          "type": "boolean"
        }
      }
    }
  },
  "ref": "Query"
}
```

## Limitations of nesting

GraphQL's introspection system doesn't make it possible to get arbitrarily-deep
arrays or non-null values. As a result, at a certain point `graphql-to-jddf`
will "bottom out", and simply emit an empty (catch-all) schema.

For example, this GraphQL schema:

```graphql
schema {
  query: Query
}

type Query {
  deep: [[[String!]!]!]!
}
```

Becomes:

```json
{
  "definitions": {
    "Query": {
      "properties": {
        "deep": {
          "elements": {
            "elements": {
              "elements": {
                "type": "string"
              }
            }
          }
        }
      }
    }
  },
  "ref": "Query"
}
```

But this GraphQL schema hits the max depth:

```graphql
schema {
  query: Query
}

type Query {
  deep: [[[[String!]!]!]!]!
}
```

And so becomes (note the lack of `{"type": "string"}`):

```json
{
  "definitions": {
    "Query": {
      "properties": {
        "deep": {
          "elements": {
            "elements": {
              "elements": {
                "elements": {}
              }
            }
          }
        }
      }
    }
  },
  "ref": "Query"
}
```

## Limitations of unions and interfaces

`graphql-to-jddf` produces less-than-ideal output for GraphQL interfaces and
unions:

```graphql
schema {
  query: Query
}

type Query {
  union: U
  interface: I
}

union U = A | B | C

interface I {
  x: String!
}

type A {
  x: String!
}

type B {
  x: String!
}

type C {
  x: String!
}
```

Becomes:

```json
{
  "definitions": {
    "Query": {
      "properties": {},
      "optionalProperties": {
        "union": {
          "ref": "U"
        },
        "interface": {
          "ref": "I"
        }
      }
    },
    "I": {},
    "U": {},
    "A": {
      "properties": {
        "x": {
          "type": "string"
        }
      }
    },
    "B": {
      "properties": {
        "x": {
          "type": "string"
        }
      }
    },
    "C": {
      "properties": {
        "x": {
          "type": "string"
        }
      }
    }
  },
  "ref": "Query"
}
```

This is a known limitation. If you have a use-case where a particular sort of
JDDF schema needs to be outputted from `union` or `interface`, please open a
GitHub ticket.
