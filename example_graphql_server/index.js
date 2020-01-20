const fs = require("fs");
const express = require("express");
const graphqlHTTP = require("express-graphql");
const { buildSchema } = require("graphql");

const schema = buildSchema(fs.readFileSync(0, "utf-8"));
const app = express();
app.use(
  "/graphql",
  graphqlHTTP({ schema, rootValue: undefined, graphiql: true })
);

app.listen(4000);

// var app = express();
// app.use(
//   "/graphql",
//   graphqlHTTP({
//     schema: schema,
//     rootValue: root,
//     graphiql: true
//   })
// );
// app.listen(4000);
// console.log("Running a GraphQL API server at http://localhost:4000/graphql");
