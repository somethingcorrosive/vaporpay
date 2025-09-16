// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

// mainnet-contract/VaporPay.sol

/// @notice Minimal ERC20 interface
interface IERC20 {
    function transferFrom(address from, address to, uint256 value) external returns (bool);
    function transfer(address to, uint256 value) external returns (bool);
}

contract VaporPay {
    struct Commitment {
        address depositor;
        address token;
        uint256 amount;
        uint256 expiry;
        bool refundable;
    }

    mapping(bytes32 => Commitment) public commitments;

    event Deposited(
        bytes32 indexed commitment,
        address indexed depositor,
        address token,
        uint256 amount,
        uint256 expiry,
        bool refundable
    );

    event Redeemed(bytes32 indexed commitment, address indexed to, address token, uint256 amount);
    event Refunded(bytes32 indexed commitment, address indexed to, address token, uint256 amount);

    // --------- DEPOSITS ---------

    function depositETH(bytes32 commitment, uint256 expiry, bool refundable) external payable {
        require(msg.value > 0, "No ETH sent");
        require(expiry > block.timestamp, "Expiry must be in future");
        require(commitments[commitment].amount == 0, "Commitment already used");

        commitments[commitment] = Commitment({
            depositor: msg.sender,
            token: address(0),
            amount: msg.value,
            expiry: expiry,
            refundable: refundable
        });

        emit Deposited(commitment, msg.sender, address(0), msg.value, expiry, refundable);
    }

    function depositERC20(
        bytes32 commitment,
        address token,
        uint256 amount,
        uint256 expiry,
        bool refundable
    ) external {
        require(amount > 0, "No tokens sent");
        require(expiry > block.timestamp, "Expiry must be in future");
        require(commitments[commitment].amount == 0, "Commitment already used");

        require(IERC20(token).transferFrom(msg.sender, address(this), amount), "ERC20 transfer failed");

        commitments[commitment] = Commitment({
            depositor: msg.sender,
            token: token,
            amount: amount,
            expiry: expiry,
            refundable: refundable
        });

        emit Deposited(commitment, msg.sender, token, amount, expiry, refundable);
    }

    // --------- REDEEM ---------

    function redeem(bytes32 secret, bytes32 salt, address to) external {
        bytes32 commitment = keccak256(abi.encodePacked(secret, salt));
        Commitment storage c = commitments[commitment];

        require(c.amount > 0, "Already redeemed/refunded");
        require(block.timestamp <= c.expiry, "Expired");

        uint256 amount = c.amount;
        address token = c.token;
        c.amount = 0;

        _transfer(token, to, amount);

        emit Redeemed(commitment, to, token, amount);
    }

    // --------- REFUND ---------

    function refund(bytes32 secret, bytes32 salt) external {
        bytes32 commitment = keccak256(abi.encodePacked(secret, salt));
        Commitment storage c = commitments[commitment];

        require(c.amount > 0, "Nothing to refund");
        require(block.timestamp > c.expiry, "Not expired");
        require(c.refundable, "Non-refundable funds cannot be recovered");

        uint256 amount = c.amount;
        address token = c.token;
        c.amount = 0;

        address recipient = c.depositor;
        _transfer(token, recipient, amount);

        emit Refunded(commitment, recipient, token, amount);
    }

    // --------- INTERNAL ---------

    function _transfer(address token, address to, uint256 amount) internal {
        if (token == address(0)) {
            (bool ok, ) = to.call{value: amount}("");
            require(ok, "ETH transfer failed");
        } else {
            require(IERC20(token).transfer(to, amount), "ERC20 transfer failed");
        }
    }
}

