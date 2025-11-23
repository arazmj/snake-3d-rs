// In-memory storage for scores.
// Note: In a serverless environment like Azure Functions, this state may be lost
// when the function app scales down or restarts. For persistent storage,
// you should use a database (e.g., Azure Cosmos DB or Table Storage).
let scores = [
    { name: "TestPlayer", score: 100 }
];

module.exports = async function (context, req) {
    context.log('JavaScript HTTP trigger function processed a request.');

    const method = req.method.toLowerCase();

    if (method === 'get') {
        // Sort descending by score
        scores.sort((a, b) => b.score - a.score);
        // Return top 10
        context.res = {
            status: 200,
            body: scores.slice(0, 10),
            headers: {
                'Content-Type': 'application/json'
            }
        };
    } else if (method === 'post') {
        const { name, score } = req.body;

        if (!name || score === undefined) {
            context.res = {
                status: 400,
                body: "Please pass a name and score in the request body"
            };
            return;
        }

        const newScore = {
            name: String(name),
            score: Number(score)
        };

        scores.push(newScore);

        context.res = {
            status: 200,
            body: "Score saved"
        };
    } else {
        context.res = {
            status: 405,
            body: "Method not allowed"
        };
    }
};
