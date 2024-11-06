const path = require('path');
const { stderr } = require('process');
const TerserPlugin = require('terser-webpack-plugin');

const filename = "framework.min.js";

module.exports = {

    entry: './index.js',
    output: {
        path: path.join(__dirname , ".."),
        filename
    },
    mode: 'production',
    performance: {
        maxAssetSize: 9999999,
        assetFilter: (asset) => {
            return asset.match(filename)
        }
    },
    optimization: {
        minimize: true,
        minimizer: [
            new TerserPlugin({
                extractComments: {
                    condition: false,
                },
                terserOptions: {
                    compress: true
                }
            })
        ]
    }
};