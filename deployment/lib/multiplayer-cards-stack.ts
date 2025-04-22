import * as cdk from 'aws-cdk-lib';
import {RemovalPolicy} from 'aws-cdk-lib';
import * as path from 'path';
import * as acm from 'aws-cdk-lib/aws-certificatemanager';
import {Construct} from 'constructs';
import {RustFunction} from 'cargo-lambda-cdk';
import {
  DomainName,
  HttpApi,
  HttpMethod,
  HttpStage,
  WebSocketApi,
  WebSocketStage
} from 'aws-cdk-lib/aws-apigatewayv2';
import {HttpLambdaIntegration, WebSocketLambdaIntegration} from 'aws-cdk-lib/aws-apigatewayv2-integrations';
import * as config from '../config.json'
import {AttributeType, BillingMode, Table} from 'aws-cdk-lib/aws-dynamodb';
import {HttpLambdaAuthorizer, WebSocketLambdaAuthorizer} from 'aws-cdk-lib/aws-apigatewayv2-authorizers';

const BASE_PATH = path.join(__dirname, '..', '..', 'backend')
const CERT_ARN = config.certificateArn

export class MultiplayerCardsStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    const table = new Table(this, 'Table', {
      partitionKey: {name: 'pk', type: AttributeType.STRING},
      readCapacity: 5,
      writeCapacity: 5,
      billingMode: BillingMode.PROVISIONED,
      removalPolicy: RemovalPolicy.DESTROY,
    })

    const domainCert = acm.Certificate.fromCertificateArn(this, 'Cert', CERT_ARN);

    const httpDomain = new DomainName(this, 'HttpDomain', {
      domainName: config.devHttpDomain,
      certificate: domainCert,
    });

    const webSocketDomain = new DomainName(this, 'WebSocketDomain', {
      domainName: config.devWebSocketDomain,
      certificate: domainCert,
    });

    const authorizerFunction = new RustFunction(this, 'lambda-authorizer', {
      manifestPath: BASE_PATH,
      binaryName: 'lambda-authorizer',
      environment: {
        'JWT_SECRET': config.jwtSecret,
      },
    });

    const websocketFunction = new RustFunction(this, 'websocket-function', {
      manifestPath: BASE_PATH,
      binaryName: 'websocket_handler',
      environment: {
        'TABLE_NAME': table.tableName,
        'JWT_SECRET': config.jwtSecret, //todo better secret storage
      },
    });
    table.grantReadWriteData(websocketFunction);

    const websocketAuthorizer = new WebSocketLambdaAuthorizer('WebSocketAuthorizer', authorizerFunction, {
      identitySource: ['route.request.header.Authorization'],
      authorizerName: "websocket-authorizer",
    });

    const httpAuthorizer = new HttpLambdaAuthorizer('HttpAuthorizer', authorizerFunction, {
      identitySource: ['$request.header.Authorization'],
      authorizerName: "http-authorizer",
    });

    // Distinct integrations required per route otherwise execute permissions aren't setup correctly (only for websockets)
    // https://github.com/aws/aws-cdk/issues/22940
    const webSocketApi = new WebSocketApi(this, 'multiplayer-cards-websocket', {
      connectRouteOptions: {integration: new WebSocketLambdaIntegration('WebSocketConnectIntegration', websocketFunction), authorizer: websocketAuthorizer},
      disconnectRouteOptions: {integration: new WebSocketLambdaIntegration('WebSocketDisconnectIntegration', websocketFunction)},
      defaultRouteOptions: {integration: new WebSocketLambdaIntegration('WebSocketDefaultIntegration', websocketFunction)},
    });

    const webSocketStage = new WebSocketStage(this, 'WebSocketDevStage', {
      webSocketApi: webSocketApi,
      stageName: 'dev',
      domainMapping: {
        domainName: webSocketDomain,
      },
      autoDeploy: true,
    });

    authorizerFunction.addEnvironment("WEBSOCKET_ARN", webSocketApi.arnForExecuteApiV2("$connect", webSocketStage.stageName))
    websocketFunction.addEnvironment('WEBSOCKET_ENDPOINT', webSocketStage.callbackUrl);
    webSocketApi.grantManageConnections(websocketFunction);

    const httpFunction = new RustFunction(this, 'http-function', {
      manifestPath: BASE_PATH,
      binaryName: 'http',
      environment: {
        'TABLE_NAME': table.tableName,
        'JWT_SECRET': config.jwtSecret, //todo better secret storage
      },
    });
    table.grantReadWriteData(httpFunction);

    const httpApi = new HttpApi(this, 'multiplayer-cards-http', {
      createDefaultStage: false,
    });

    const httpIntegration = new HttpLambdaIntegration('HttpIntegration', httpFunction);

    httpApi.addRoutes({
      path: '/auth/guest',
      methods: [ HttpMethod.POST ],
      integration: httpIntegration,
    });
    httpApi.addRoutes({
      path: '/auth/refresh',
      methods: [ HttpMethod.POST ],
      integration: httpIntegration,
    });

    // Authorised routes
    httpApi.addRoutes({
      path: '/game/create',
      methods: [ HttpMethod.POST ],
      authorizer: httpAuthorizer,
      integration: httpIntegration,
    });
    httpApi.addRoutes({
      path: '/game/join',
      methods: [ HttpMethod.POST ],
      authorizer: httpAuthorizer,
      integration: httpIntegration,
    });

    new HttpStage(this, 'HttpDevStage', {
      httpApi: httpApi,
      domainMapping: {
        domainName: httpDomain,
      },
      stageName: 'dev',
      autoDeploy: true,
    });
  }
}
