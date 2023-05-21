const CopyWebpackPlugin = require("copy-webpack-plugin");
const path = require('path');

module.exports = {
    entry: "./index.js",
    output: {
        path: path.resolve(__dirname, "dist"),
        filename: "index.js",
    },
    module: {
        rules: [{
            test: /\.css$/,
            use: [
                'style-loader',
                'css-loader'
            ]
        }]
    },
    plugins: [
        new CopyWebpackPlugin({
            patterns: [
                // { from: 'assets/*.svg' },
                { from: 'index.html' },
            ]
        }),
    ],
    experiments: {
        asyncWebAssembly: true,
        topLevelAwait: true,
    },
};