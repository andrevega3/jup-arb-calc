# Multi-Hop Currency Conversion and Arbitrage Calculator on Jupiter (WIP)

## Overview
This project implements a system to evaluate and compare the effectiveness of multi-hop currency conversion routes versus direct conversion routes in a Jupiter aggregator.

The project's core functionality includes:
- Fetching real-time currency conversion rates.
- Calculating the cumulative output of currencies through multiple conversion steps.
- Comparing the final amounts from multi-hop routes to determine the most cost-effective path.
- Providing tools to analyze potential arbitrage opportunities in currency exchanges.

## Features
- **Real-Time Data Fetching**: Connects to Jupiter APIs to retrieve up-to-date conversion rates.
- **Arbitrage Detection**: Identifies and highlights profitable arbitrage opportunities across different currency pairs.
- **Efficiency Comparisons**: Compares multi-hop conversion efficiencies against direct conversions.

## TODO
- Take into account slippage

## Technologies Used
- **Rust**: Chosen for its performance, reliability, and concurrency features.

## Installation
To set up this project locally, follow these steps:

1. **Clone the Repository**
   ```bash
   git clone https://github.com/andrevega3/jup-arb-calc.git
   cd jup-arb-calc
