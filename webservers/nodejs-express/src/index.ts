import express from "express";

const app = express();
const port = 8000;

app.get("/benchmark/health", (_req, res) => {
	res.send({});
});

app.get("/benchmark/plain-text", (_req, res) => {
	res.send("Hello, World!");
});

app.get("/benchmark/download-binary", (_req, res) => {
	res.type("image/png");
	res.sendFile("/assets/download-binary.png");
});

app.listen(port, () => {
	console.log(`app listening on port ${port}`);
});
