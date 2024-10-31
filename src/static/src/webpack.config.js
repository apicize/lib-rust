const webpack = require('webpack');
const UglifyPlugin = require('uglifyjs-webpack-plugin');

const PROD = JSON.parse(process.env.PROD_ENV || '0');

module.exports = {

    entry: './index.js',
    //   devtool: 'source-map',
    output: {
        path: __dirname + '/..',
        filename: 'framework.min.js'
    },
    mode: 'production',
    optimization: {
        minimizer: [
            new UglifyPlugin({
                extractComments: false,
                uglifyOptions: {
                    output: {
                        comments: false,
                        beautify: false,
                        
                    },
                    mangle: false,
                    compress: false,
                }
            })
        ]
    }
};