const TerserPlugin = require('terser-webpack-plugin');
// const UglifyPlugin = require('uglifyjs-webpack-plugin');

const PROD = JSON.parse(process.env.PROD_ENV || '0');

module.exports = {

    entry: './index.js',
    //   devtool: 'source-map',
    output: {
        path: __dirname + '/..',
        filename: 'framework.min.js'
    },
    mode: 'production',
    performance: {
        maxAssetSize: 9999999,
        assetFilter: (asset) => {
            return asset.match('framwork.min.js')
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
        //     minimizer: [
        //         new UglifyPlugin({
        //             extractComments: false,
        //             uglifyOptions: {
        //                 output: {
        //                     comments: false,
        //                     beautify: false,

        //                 },
        //                 mangle: false,
        //                 compress: false,
        //             }
        //         })
        //     ]
    }
};