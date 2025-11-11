import express from "express";

const app = express();
const port = 8000;

app.get("/benchmark/health", (_req, res) => {
	res.send({});
});

app.get("/benchmark/plain-text", (_req, res) => {
	res.send("Hello, World!");
});

app.listen(port, () => {
	console.log(`app listening on port ${port}`);
});
